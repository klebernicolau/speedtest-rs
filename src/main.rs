use anyhow::{Context, Result};
use clap::Parser;
use reqwest::blocking::Client;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use std::io::{self, Read};

use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph, Gauge},
    layout::{Layout, Constraint, Direction},
    Terminal,
    style::{Color, Style, Modifier},
};
use crossterm::{execute, event::{self, Event, KeyCode}, terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};

#[derive(Parser, Debug)]
#[command(author, version, about = "Auditor Speedtest - Engine 64KB", long_about = None)]
struct Cli {
    #[arg(short = 'd', long = "duration", default_value = "10")]
    duration: u64,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    tui: bool,
}

#[derive(Serialize)]
struct FinalReport {
    timestamp: String,
    server_name: String,
    sponsor: String,
    ping_ms: f64,
    download_mbps: f64,
    upload_mbps: f64,
}

const OOKLA_SERVERS_URL: &str = "https://www.speedtest.net/api/js/servers?engine=js";
const DOWNLOAD_SIZE: usize = 500_000_000; 
const UPLOAD_CHUNK: usize = 8_388_608; 

struct OoklaServer {
    url: String,
    name: String,
    sponsor: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(64)
        .build()?;

    let server = get_best_server(&client)?;
    let ping = measure_ping(&client, &server.url)?;

    if cli.json {
        let (dl_mbps, _) = test_process_silent(&client, &server.url, cli.duration, true)?;
        let (ul_mbps, _) = test_process_silent(&client, &server.url, cli.duration, false)?;
        let report = FinalReport {
            timestamp: chrono::Local::now().to_rfc3339(),
            server_name: server.name,
            sponsor: server.sponsor,
            ping_ms: ping,
            download_mbps: dl_mbps,
            upload_mbps: ul_mbps,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    if cli.tui {
        run_tui_mode(&client, &server, ping, cli.duration)?;
        return Ok(());
    }

    println!("  Servidor : {} ({})", server.sponsor, server.name);
    let (dl_mbps, _) = test_process(&client, &server.url, cli.duration, true)?;
    let (ul_mbps, _) = test_process(&client, &server.url, cli.duration, false)?;
    println!("\n  Download: {:.2} Mbps | Upload: {:.2} Mbps", dl_mbps, ul_mbps);
    
    Ok(())
}

fn test_process(client: &Client, base_url: &str, duration: u64, is_dl: bool) -> Result<(f64, usize)> {
    let total_bytes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    let start = Instant::now();
    let base_path = base_url.rsplitn(2, '/').collect::<Vec<&str>>()[1];
    let mut handles = Vec::new();

    for _ in 0..num_cpus::get().min(8) {
        let t = Arc::clone(&total_bytes);
        let r = Arc::clone(&running);
        let c = client.clone();
        let url = if is_dl { format!("{}/download?size={}", base_path, DOWNLOAD_SIZE) } else { format!("{}/upload", base_path) };
        handles.push(thread::spawn(move || {
            let mut buf = [0u8; 65536]; 
            while r.load(Ordering::Relaxed) {
                if is_dl {
                    if let Ok(mut resp) = c.get(&url).send() {
                        while let Ok(n) = resp.read(&mut buf) {
                            if n == 0 || !r.load(Ordering::Relaxed) { break; }
                            t.fetch_add(n as u64, Ordering::Relaxed);
                        }
                    }
                } else {
                    let data = vec![0u8; UPLOAD_CHUNK];
                    if let Ok(res) = c.post(&url).body(data).send() {
                        if res.status().is_success() { t.fetch_add(UPLOAD_CHUNK as u64, Ordering::Relaxed); }
                    }
                }
            }
        }));
    }
    thread::sleep(Duration::from_secs(duration));
    running.store(false, Ordering::Relaxed);
    for h in handles { let _ = h.join(); }
    let final_bytes = total_bytes.load(Ordering::Relaxed) as usize;
    let final_mbps = (final_bytes as f64 * 8.0 / start.elapsed().as_secs_f64()) / 1_000_000.0;
    Ok((final_mbps, final_bytes))
}

fn test_process_silent(client: &Client, base_url: &str, duration: u64, is_dl: bool) -> Result<(f64, usize)> {
    test_process(client, base_url, duration, is_dl)
}

fn run_tui_mode(client: &Client, server: &OoklaServer, ping: f64, duration: u64) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let dl_mbps = Arc::new(AtomicU64::new(0));
    let ul_mbps = Arc::new(AtomicU64::new(0));
    let dl_progress = Arc::new(AtomicU64::new(0));
    let ul_progress = Arc::new(AtomicU64::new(0));
    let is_finished = Arc::new(AtomicBool::new(false));

    let t_dl_mbps = Arc::clone(&dl_mbps);
    let t_ul_mbps = Arc::clone(&ul_mbps);
    let t_dl_progress = Arc::clone(&dl_progress);
    let t_ul_progress = Arc::clone(&ul_progress);
    let t_is_finished = Arc::clone(&is_finished);
    
    let client_thread = client.clone();
    let server_url = server.url.clone();

    thread::spawn(move || {
        let _ = test_process_tui(&client_thread, &server_url, duration, true, t_dl_mbps, t_dl_progress);
        let _ = test_process_tui(&client_thread, &server_url, duration, false, t_ul_mbps, t_ul_progress);
        t_is_finished.store(true, Ordering::SeqCst);
    });

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0)
                ].as_ref())
                .split(f.size());

            let header = Paragraph::new(format!(" Auditoria: {} | Host: {} | Ping: {:.1}ms", server.sponsor, server.name, ping))
                .block(Block::default().borders(Borders::ALL).title(" Speedtest-RS Auditoria "));

            let dl_val = dl_mbps.load(Ordering::Relaxed) as f64 / 100.0;
            let dl_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title(format!(" 📥 Download: {:.2} Mbps ", dl_val)))
                .gauge_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .ratio((dl_progress.load(Ordering::Relaxed) as f64 / 100.0).min(1.0));

            let ul_val = ul_mbps.load(Ordering::Relaxed) as f64 / 100.0;
            let ul_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title(format!(" 📤 Upload: {:.2} Mbps ", ul_val)))
                .gauge_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                .ratio((ul_progress.load(Ordering::Relaxed) as f64 / 100.0).min(1.0));

            let status = if is_finished.load(Ordering::SeqCst) {
                " [ AUDITORIA FINALIZADA ] - Pressione Ctrl+C para sair."
            } else {
                " [ PROCESSANDO ENGINE 64KB... ] "
            };
            
            f.render_widget(header, chunks[0]);
            f.render_widget(dl_gauge, chunks[1]);
            f.render_widget(ul_gauge, chunks[2]);
            f.render_widget(Paragraph::new(status), chunks[3]);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn test_process_tui(client: &Client, base_url: &str, duration: u64, is_dl: bool, mbps_atom: Arc<AtomicU64>, prog_atom: Arc<AtomicU64>) -> Result<()> {
    let total_bytes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    let start = Instant::now();
    let base_path = base_url.rsplitn(2, '/').collect::<Vec<&str>>()[1];
    
    let mut handles = Vec::new();
    for _ in 0..num_cpus::get().min(8) {
        let t = Arc::clone(&total_bytes);
        let r = Arc::clone(&running);
        let c = client.clone();
        let url = if is_dl { format!("{}/download?size={}", base_path, DOWNLOAD_SIZE) } else { format!("{}/upload", base_path) };
        handles.push(thread::spawn(move || {
            let mut buf = [0u8; 65536];
            while r.load(Ordering::Relaxed) {
                if is_dl {
                    if let Ok(mut resp) = c.get(&url).send() {
                        while let Ok(n) = resp.read(&mut buf) {
                            if n == 0 || !r.load(Ordering::Relaxed) { break; }
                            t.fetch_add(n as u64, Ordering::Relaxed);
                        }
                    }
                } else {
                    let data = vec![0u8; UPLOAD_CHUNK];
                    if let Ok(res) = c.post(&url).body(data).send() {
                        if res.status().is_success() { t.fetch_add(UPLOAD_CHUNK as u64, Ordering::Relaxed); }
                    }
                }
            }
        }));
    }

    while start.elapsed().as_secs() < duration {
        thread::sleep(Duration::from_millis(500));
        let bytes = total_bytes.load(Ordering::Relaxed);
        let elapsed = start.elapsed().as_secs_f64();
        if elapsed > 0.1 {
            let mbps = (bytes as f64 * 8.0 / elapsed) / 1_000_000.0;
            mbps_atom.store((mbps * 100.0) as u64, Ordering::Relaxed);
        }
        prog_atom.store(((elapsed / duration as f64) * 100.0) as u64, Ordering::Relaxed);
    }

    running.store(false, Ordering::SeqCst);
    for h in handles { let _ = h.join(); }
    
    // Ajuste final para precisão máxima após join das threads
    let final_bytes = total_bytes.load(Ordering::Relaxed);
    let final_mbps = (final_bytes as f64 * 8.0 / start.elapsed().as_secs_f64()) / 1_000_000.0;
    mbps_atom.store((final_mbps * 100.0) as u64, Ordering::SeqCst);
    prog_atom.store(100, Ordering::SeqCst);
    
    Ok(())
}

fn get_best_server(client: &Client) -> Result<OoklaServer> {
    let text = client.get(OOKLA_SERVERS_URL).send()?.text()?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    let s = &json[0];
    Ok(OoklaServer {
        url: s["url"].as_str().context("URL")?.to_string(),
        name: s["name"].as_str().context("Name")?.to_string(),
        sponsor: s["sponsor"].as_str().context("Sponsor")?.to_string(),
    })
}

fn measure_ping(client: &Client, url: &str) -> Result<f64> {
    let base = url.rsplitn(2, '/').collect::<Vec<&str>>()[1];
    let ping_url = format!("{}/ping.php", base);
    let start = Instant::now();
    let _ = client.get(ping_url).send()?;
    Ok(start.elapsed().as_secs_f64() * 1000.0)
}
