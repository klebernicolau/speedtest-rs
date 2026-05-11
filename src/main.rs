use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
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
#[command(author, version, about = "Speedtest Auditor - Engine 64KB")]
struct Cli {
    #[arg(short = 'd', long = "duration", default_value = "10")]
    duration: u64,
    #[arg(long)]
    tui: bool,
}

const OOKLA_SERVERS_URL: &str = "https://www.speedtest.net/api/js/servers?engine=js";
const DOWNLOAD_SIZE: usize = 50_000_000;
const UPLOAD_CHUNK: usize = 1_048_576;

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

    if cli.tui {
        run_tui_mode(&client, &server, ping, cli.duration)?;
    } else {
        println!("\n========================================");
        println!(" Servidor : {} ({})", server.sponsor, server.name);
        println!(" Ping     : {:.1} ms", ping);
        println!("========================================");
        
        let (dl_mbps, _) = run_cli_test(&client, &server.url, cli.duration, true)?;
        let (ul_mbps, _) = run_cli_test(&client, &server.url, cli.duration, false)?;
        
        println!("\n========================================");
        println!(" 📊 RESULTADOS");
        println!(" DL: {:.2} Mbps | UL: {:.2} Mbps", dl_mbps, ul_mbps);
        println!("========================================\n");
    }

    Ok(())
}

fn strip_path(url: &str) -> String {
    let parts: Vec<&str> = url.rsplitn(2, '/').collect();
    parts[1].to_string()
}

fn run_cli_test(client: &Client, base_url: &str, duration: u64, is_dl: bool) -> Result<(f64, usize)> {
    let label = if is_dl { "DL" } else { "UL" };
    
    // BARRA CURTA (20 chars) e TUDO NA MESMA LINHA
    let pb = ProgressBar::new(duration * 10);
    pb.set_style(ProgressStyle::default_bar()
        .template("{prefix} [{bar:20.cyan/blue}] {percent}% {msg}")?
        .progress_chars("#>-"));
    
    pb.set_prefix(format!(" {}", label));
    pb.set_message("0.00 Mbps");

    let base = strip_path(base_url);
    let total_bytes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..num_cpus::get().min(8) {
        let t = Arc::clone(&total_bytes);
        let r = Arc::clone(&running);
        let c = client.clone();
        let url = if is_dl { format!("{}/download?size={}", base, DOWNLOAD_SIZE) } else { format!("{}/upload", base) };
        
        handles.push(thread::spawn(move || {
            let data = vec![0u8; UPLOAD_CHUNK];
            while r.load(Ordering::Relaxed) {
                if is_dl {
                    if let Ok(mut resp) = c.get(&url).send() {
                        let mut buf = [0u8; 16384];
                        while let Ok(n) = resp.read(&mut buf) {
                            if n == 0 || !r.load(Ordering::Relaxed) { break; }
                            t.fetch_add(n as u64, Ordering::Relaxed);
                        }
                    }
                } else {
                    if let Ok(res) = c.post(&url).body(data.clone()).send() {
                        if res.status().is_success() { t.fetch_add(UPLOAD_CHUNK as u64, Ordering::Relaxed); }
                    }
                }
            }
        }));
    }

    while start.elapsed().as_secs() < duration {
        let elapsed = start.elapsed().as_secs_f64();
        let bytes = total_bytes.load(Ordering::Relaxed);
        let mbps = (bytes as f64 * 8.0 / elapsed.max(0.1)) / 1_000_000.0;
        
        pb.set_position((elapsed * 10.0) as u64);
        pb.set_message(format!("| {:.2} Mbps", mbps));
        thread::sleep(Duration::from_millis(100));
    }

    running.store(false, Ordering::SeqCst);
    for h in handles { let _ = h.join(); }

    let final_mbps = (total_bytes.load(Ordering::Relaxed) as f64 * 8.0 / start.elapsed().as_secs_f64()) / 1_000_000.0;
    pb.set_message(format!("| {:.2} Mbps [OK]", final_mbps));
    pb.finish();

    Ok((final_mbps, total_bytes.load(Ordering::Relaxed) as usize))
}

// --- MANTENDO TUI ---
fn run_tui_mode(client: &Client, server: &OoklaServer, ping: f64, duration: u64) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let dl_mbps = Arc::new(AtomicU64::new(0));
    let ul_mbps = Arc::new(AtomicU64::new(0));
    let dl_prog = Arc::new(AtomicU64::new(0));
    let ul_prog = Arc::new(AtomicU64::new(0));
    let finished = Arc::new(AtomicBool::new(false));

    let (t_dl, t_ul, t_dp, t_up, t_f) = (Arc::clone(&dl_mbps), Arc::clone(&ul_mbps), Arc::clone(&dl_prog), Arc::clone(&ul_prog), Arc::clone(&finished));
    let c_thread = client.clone();
    let s_url = server.url.clone();

    thread::spawn(move || {
        let _ = test_process_tui_logic(&c_thread, &s_url, duration, true, t_dl, t_dp);
        let _ = test_process_tui_logic(&c_thread, &s_url, duration, false, t_ul, t_up);
        t_f.store(true, Ordering::SeqCst);
    });

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(f.size());

            f.render_widget(Paragraph::new(format!(" Servidor: {} | Ping: {:.1}ms", server.sponsor, ping)).block(Block::default().borders(Borders::ALL).title(" Speedtest Auditor ")), chunks[0]);
            f.render_widget(Gauge::default().block(Block::default().borders(Borders::ALL).title(format!(" 📥 Download: {:.2} Mbps ", dl_mbps.load(Ordering::Relaxed) as f64 / 100.0))).gauge_style(Style::default().fg(Color::Cyan)).ratio((dl_prog.load(Ordering::Relaxed) as f64 / 100.0).min(1.0)), chunks[1]);
            f.render_widget(Gauge::default().block(Block::default().borders(Borders::ALL).title(format!(" 📤 Upload: {:.2} Mbps ", ul_mbps.load(Ordering::Relaxed) as f64 / 100.0))).gauge_style(Style::default().fg(Color::Green)).ratio((ul_prog.load(Ordering::Relaxed) as f64 / 100.0).min(1.0)), chunks[2]);
            
            let status = if finished.load(Ordering::SeqCst) { " [ FINALIZADO ] - Ctrl+C para sair." } else { " [ EXECUTANDO... ] " };
            f.render_widget(Paragraph::new(status).style(Style::default().add_modifier(Modifier::BOLD)), chunks[3]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) { break; }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn test_process_tui_logic(client: &Client, base_url: &str, duration: u64, is_dl: bool, mbps_atom: Arc<AtomicU64>, prog_atom: Arc<AtomicU64>) -> Result<()> {
    let base = strip_path(base_url);
    let total_bytes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..num_cpus::get().min(8) {
        let t = Arc::clone(&total_bytes);
        let r = Arc::clone(&running);
        let c = client.clone();
        let url = if is_dl { format!("{}/download?size={}", base, DOWNLOAD_SIZE) } else { format!("{}/upload", base) };
        handles.push(thread::spawn(move || {
            let data = vec![0u8; UPLOAD_CHUNK];
            while r.load(Ordering::Relaxed) {
                if is_dl {
                    if let Ok(mut resp) = c.get(&url).send() {
                        let mut buf = [0u8; 16384];
                        while let Ok(n) = resp.read(&mut buf) {
                            if n == 0 || !r.load(Ordering::Relaxed) { break; }
                            t.fetch_add(n as u64, Ordering::Relaxed);
                        }
                    }
                } else {
                    if let Ok(res) = c.post(&url).body(data.clone()).send() {
                        if res.status().is_success() { t.fetch_add(UPLOAD_CHUNK as u64, Ordering::Relaxed); }
                    }
                }
            }
        }));
    }

    while start.elapsed().as_secs() < duration {
        let bytes = total_bytes.load(Ordering::Relaxed);
        let elapsed = start.elapsed().as_secs_f64();
        if elapsed > 0.1 {
            let mbps = (bytes as f64 * 8.0 / elapsed) / 1_000_000.0;
            mbps_atom.store((mbps * 100.0) as u64, Ordering::Relaxed);
        }
        prog_atom.store(((elapsed / duration as f64) * 100.0) as u64, Ordering::Relaxed);
        thread::sleep(Duration::from_millis(200));
    }

    running.store(false, Ordering::SeqCst);
    for h in handles { let _ = h.join(); }
    prog_atom.store(100, Ordering::SeqCst);
    Ok(())
}

fn get_best_server(client: &Client) -> Result<OoklaServer> {
    let text = client.get(OOKLA_SERVERS_URL).send()?.text()?;
    let json: Vec<serde_json::Value> = serde_json::from_str(&text)?;
    let s = &json[0];
    Ok(OoklaServer {
        url: s["url"].as_str().context("URL")?.to_string(),
        name: s["name"].as_str().context("Name")?.to_string(),
        sponsor: s["sponsor"].as_str().context("Sponsor")?.to_string(),
    })
}

fn measure_ping(client: &Client, url: &str) -> Result<f64> {
    let ping_url = format!("{}/ping.php", strip_path(url));
    let start = Instant::now();
    let _ = client.get(ping_url).send()?;
    Ok(start.elapsed().as_secs_f64() * 1000.0)
}
