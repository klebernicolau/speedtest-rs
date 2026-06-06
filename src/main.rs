use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use std::io::Read; // Removido o 'self' que causava o warning

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
    /// Salvar resultado em arquivo (ex: --output relatorio.json ou relatorio.csv)
    #[arg(short = 'o', long = "output")]
    output: Option<String>,
}

const OOKLA_SERVERS_URL: &str = "https://www.speedtest.net/api/js/servers?engine=js";
const DOWNLOAD_SIZE: usize = 50_000_000;
const UPLOAD_CHUNK: usize = 1_048_576;

/// Gera um buffer de bytes pseudo-aleatórios usando xorshift64.
/// Evita que servidores comprimam os dados no upload, garantindo medição real.
fn random_payload(size: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size);
    let mut state: u64 = 0xDEAD_BEEF_CAFE_1337;
    while buf.len() < size {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        buf.extend_from_slice(&state.to_le_bytes());
    }
    buf.truncate(size);
    buf
}

struct OoklaServer {
    url: String,
    name: String,
    sponsor: String,
}

use std::fs::OpenOptions;
use std::io::Write;

struct AuditRecord {
    timestamp: String,
    server_sponsor: String,
    server_name: String,
    ping_avg_ms: f64,
    ping_min_ms: f64,
    ping_max_ms: f64,
    jitter_ms: f64,
    dl_mbps: f64,
    ul_mbps: f64,
}

fn now_iso() -> String {
    // Timestamp simples sem dependência de chrono
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Converte UNIX timestamp para formato legível
    let secs  = d % 60;
    let mins  = (d / 60) % 60;
    let hours = (d / 3600) % 24;
    let days  = d / 86400;
    // Aproximação de data a partir do epoch (suficiente para auditoria)
    format!("epoch+{}d {:02}:{:02}:{:02}Z", days, hours, mins, secs)
}

fn save_audit(path: &str, rec: &AuditRecord) -> Result<()> {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    let line = if ext == "csv" {
        let file_exists = std::path::Path::new(path).exists();
        let header = if !file_exists {
            "timestamp,server_sponsor,server_name,ping_avg_ms,ping_min_ms,ping_max_ms,jitter_ms,dl_mbps,ul_mbps\n".to_string()
        } else {
            String::new()
        };
        format!(
            "{}{},{},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}\n",
            header,
            rec.timestamp, rec.server_sponsor, rec.server_name,
            rec.ping_avg_ms, rec.ping_min_ms, rec.ping_max_ms,
            rec.jitter_ms, rec.dl_mbps, rec.ul_mbps
        )
    } else {
        // JSON — append em array
        let existing = std::fs::read_to_string(path).unwrap_or_else(|_| "[]".to_string());
        let trimmed = existing.trim().trim_end_matches(']').trim_end_matches(',').trim().to_string();
        let sep = if trimmed == "[" || trimmed.is_empty() { "" } else { "," };
        let prefix = if trimmed.is_empty() { "[".to_string() } else { format!("{}{}\n", trimmed, sep) };
        let entry = format!(
            concat!(
                "{{\n",
                "  \"timestamp\": \"{}\",\n",
                "  \"server_sponsor\": \"{}\",\n",
                "  \"server_name\": \"{}\",\n",
                "  \"ping_avg_ms\": {:.2},\n",
                "  \"ping_min_ms\": {:.2},\n",
                "  \"ping_max_ms\": {:.2},\n",
                "  \"jitter_ms\": {:.2},\n",
                "  \"dl_mbps\": {:.2},\n",
                "  \"ul_mbps\": {:.2}\n",
                "}}\n]\n"
            ),
            rec.timestamp, rec.server_sponsor, rec.server_name,
            rec.ping_avg_ms, rec.ping_min_ms, rec.ping_max_ms,
            rec.jitter_ms, rec.dl_mbps, rec.ul_mbps
        );
        format!("{}{}", prefix, entry)
    };

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(ext != "csv")
        .append(ext == "csv")
        .open(path)?;
    file.write_all(line.as_bytes())?;
    println!(" Resultado salvo em: {}", path);
    Ok(())
}


fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(64)
        .build()?;

    let server = get_best_server(&client)?;

    println!("\n Medindo ping (10 amostras)...");
    let ping_result = measure_ping_full(&client, &server.url, 10)?;

    if cli.tui {
        run_tui_mode(&client, &server, ping_result.avg_ms, ping_result.jitter_ms, cli.duration)?;
    } else {
        // BORDA PADRONIZADA 44 CHARS
        println!("\n==================================================");
        println!(" Servidor : {} ({})", server.sponsor, server.name);
        println!("--------------------------------------------------");
        println!(" Ping Avg : {:.1} ms", ping_result.avg_ms);
        println!(" Ping Min : {:.1} ms", ping_result.min_ms);
        println!(" Ping Max : {:.1} ms", ping_result.max_ms);
        println!(" Jitter   : {:.1} ms", ping_result.jitter_ms);
        println!("==================================================");

        println!(""); 
        let (dl_mbps, _) = run_cli_test(&client, &server.url, cli.duration, true)?;
        let (ul_mbps, _) = run_cli_test(&client, &server.url, cli.duration, false)?;

        println!("\n==================================================");
        println!(" 📊 RESULTADOS");
        println!(" DL: {:.2} Mbps | UL: {:.2} Mbps", dl_mbps, ul_mbps);
        println!("==================================================\n");

        if let Some(ref path) = cli.output {
            let rec = AuditRecord {
                timestamp: now_iso(),
                server_sponsor: server.sponsor.clone(),
                server_name: server.name.clone(),
                ping_avg_ms: ping_result.avg_ms,
                ping_min_ms: ping_result.min_ms,
                ping_max_ms: ping_result.max_ms,
                jitter_ms: ping_result.jitter_ms,
                dl_mbps,
                ul_mbps,
            };
            save_audit(path, &rec)?;
        }
    }

    Ok(())
}

fn strip_path(url: &str) -> String {
    let parts: Vec<&str> = url.rsplitn(2, '/').collect();
    parts[1].to_string()
}

fn run_cli_test(client: &Client, base_url: &str, duration: u64, is_dl: bool) -> Result<(f64, usize)> {
    let label = if is_dl { "DL" } else { "UL" };

    let pb = ProgressBar::new(duration * 10);
    pb.set_style(ProgressStyle::default_bar()
        .template(" {prefix} [{bar:18.cyan/blue}] {percent}% {msg}")?
        .progress_chars("#>-"));

    pb.set_prefix(label);
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
            // Payload aleatório: impede compressão HTTP no upload
            let data = random_payload(UPLOAD_CHUNK);
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

fn run_tui_mode(client: &Client, server: &OoklaServer, ping: f64, jitter: f64, duration: u64) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
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

            f.render_widget(Paragraph::new(format!(" Servidor: {} | Ping: {:.1}ms | Jitter: {:.1}ms", server.sponsor, ping, jitter)).block(Block::default().borders(Borders::ALL).title(" Speedtest Auditor ")), chunks[0]);
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
            // Payload aleatório: impede compressão HTTP no upload
            let data = random_payload(UPLOAD_CHUNK);
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
    println!(" Buscando servidores disponíveis...");
    let text = client.get(OOKLA_SERVERS_URL).send()?.text()?;
    let json: Vec<serde_json::Value> = serde_json::from_str(&text)?;

    // Pega até 5 candidatos da lista
    let candidates: Vec<(String, String, String)> = json.iter()
        .take(5)
        .filter_map(|s| {
            let url     = s["url"].as_str()?.to_string();
            let name    = s["name"].as_str()?.to_string();
            let sponsor = s["sponsor"].as_str()?.to_string();
            Some((url, name, sponsor))
        })
        .collect();

    anyhow::ensure!(!candidates.is_empty(), "Nenhum servidor encontrado");

    println!(" Testando latência em {} servidores...", candidates.len());

    // Testa cada candidato em thread separada (1 warmup + 3 amostras)
    let mut handles = Vec::new();
    for (url, name, sponsor) in candidates {
        let c = client.clone();
        handles.push(thread::spawn(move || -> Option<(f64, String, String, String)> {
            let ping_url = format!("{}/ping.php", strip_path(&url));
            let _ = c.get(&ping_url).send(); // warmup
            thread::sleep(Duration::from_millis(30));
            let mut times = Vec::new();
            for _ in 0..3 {
                let t = Instant::now();
                if c.get(&ping_url).send().is_ok() {
                    times.push(t.elapsed().as_secs_f64() * 1000.0);
                }
                thread::sleep(Duration::from_millis(50));
            }
            if times.is_empty() { return None; }
            let avg = times.iter().sum::<f64>() / times.len() as f64;
            Some((avg, url, name, sponsor))
        }));
    }

    // Coleta resultados e escolhe o menor ping
    let mut best: Option<(f64, String, String, String)> = None;
    for h in handles {
        if let Ok(Some(result)) = h.join() {
            println!("   {:.1}ms — {} ({})", result.0, result.3, result.2);
            if best.as_ref().map_or(true, |b| result.0 < b.0) {
                best = Some(result);
            }
        }
    }

    let (_, url, name, sponsor) = best.context("Nenhum servidor respondeu ao ping")?;
    println!(" Servidor selecionado: {} ({})", sponsor, name);
    Ok(OoklaServer { url, name, sponsor })
}

struct PingResult {
    avg_ms: f64,
    min_ms: f64,
    max_ms: f64,
    jitter_ms: f64,
}

fn measure_ping_full(client: &Client, url: &str, count: usize) -> Result<PingResult> {
    let ping_url = format!("{}/ping.php", strip_path(url));
    let mut samples: Vec<f64> = Vec::with_capacity(count);

    // Warmup - descarta a primeira amostra (conexão TCP fria)
    let _ = client.get(&ping_url).send();
    thread::sleep(Duration::from_millis(50));

    for _ in 0..count {
        let start = Instant::now();
        if client.get(&ping_url).send().is_ok() {
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        thread::sleep(Duration::from_millis(100));
    }

    if samples.is_empty() {
        anyhow::bail!("Nenhuma amostra de ping coletada");
    }

    let avg_ms = samples.iter().sum::<f64>() / samples.len() as f64;
    let min_ms = samples.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_ms = samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    // Jitter = média das diferenças absolutas entre amostras consecutivas
    let jitter_ms = if samples.len() > 1 {
        let diffs: Vec<f64> = samples.windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .collect();
        diffs.iter().sum::<f64>() / diffs.len() as f64
    } else {
        0.0
    };

    Ok(PingResult { avg_ms, min_ms, max_ms, jitter_ms })
}