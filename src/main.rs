use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short = 'd', long = "duration", default_value = "10")]
    duration: u64,
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

    println!("================================================ ");
    println!("                  Speedtest                      ");
    println!("================================================ ");
    println!("  Servidor : {} ({})", server.sponsor, server.name);
    println!("  Ping     : {:.1} ms", ping);
    println!("================================================\n");

    let (dl_mbps, dl_bytes) = test_process(&client, &server.url, cli.duration, true)?;
    let (ul_mbps, ul_bytes) = test_process(&client, &server.url, cli.duration, false)?;

    println!("\n================================================");
    println!("  📊 RESULTADOS FINAIS");
    println!("================================================");
    println!("  📥 Download : \x1b[1;32m{:.2} Mbps\x1b[0m ({})", dl_mbps, format_bytes(dl_bytes));
    println!("  📤 Upload   : \x1b[1;34m{:.2} Mbps\x1b[0m ({})", ul_mbps, format_bytes(ul_bytes));
    println!("  ⏱️  Ping     : {:.1} ms", ping);
    println!("================================================\n");

    Ok(())
}

fn test_process(client: &Client, base_url: &str, duration: u64, is_dl: bool) -> Result<(f64, usize)> {
    let label = if is_dl { "Download" } else { "Upload" };
    let icon = if is_dl { "📥" } else { "📤" };
    let color = if is_dl { "cyan" } else { "green" };

    // Cabeçalho do teste fixo
    println!("  {} Teste de {}:", icon, label);

    // Barra de progresso formatada para ficar EMBAIXO e não pular
    let pb = ProgressBar::new(duration);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(&format!("    [{{bar:30.{}}}] {{msg:>10}}", color))?
            .progress_chars("█▉▊▋▌▍▎▏  "), // Visual bonito que você preferiu
    );

    let total_bytes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    let start = Instant::now();

    let base_path = base_url.rsplitn(2, '/').collect::<Vec<&str>>()[1];
    let num_threads = num_cpus::get().min(8);
    let mut handles = Vec::new();

    for _ in 0..num_threads {
        let t = Arc::clone(&total_bytes);
        let r = Arc::clone(&running);
        let c = client.clone();
        let url = if is_dl {
            format!("{}/download?size={}", base_path, DOWNLOAD_SIZE)
        } else {
            format!("{}/upload", base_path)
        };

        handles.push(thread::spawn(move || {
            let data = vec![0u8; UPLOAD_CHUNK];
            while r.load(Ordering::Relaxed) {
                if is_dl {
                    if let Ok(mut resp) = c.get(&url).send() {
                        let mut buf = [0u8; 65536];
                        while let Ok(n) = std::io::Read::read(&mut resp, &mut buf) {
                            if n == 0 || !r.load(Ordering::Relaxed) { break; }
                            t.fetch_add(n as u64, Ordering::Relaxed);
                        }
                    }
                } else {
                    if let Ok(res) = c.post(&url).body(data.clone()).send() {
                        if res.status().is_success() { 
                            t.fetch_add(UPLOAD_CHUNK as u64, Ordering::Relaxed); 
                        }
                    }
                }
            }
        }));
    }

    for i in 0..duration {
        thread::sleep(Duration::from_secs(1));
        let bytes = total_bytes.load(Ordering::Relaxed);
        let mbps = (bytes as f64 * 8.0 / start.elapsed().as_secs_f64()) / 1_000_000.0;
        
        pb.set_position(i + 1);
        pb.set_message(format!("{:.2} Mbps", mbps));
    }

    running.store(false, Ordering::Relaxed);
    for h in handles { let _ = h.join(); }

    let final_elapsed = start.elapsed().as_secs_f64();
    let final_bytes = total_bytes.load(Ordering::Relaxed) as usize;
    let final_mbps = (final_bytes as f64 * 8.0 / final_elapsed) / 1_000_000.0;

    pb.finish_and_clear(); 
    // Move o cursor para cima para substituir a barra pelo resultado final limpo
    println!("\x1b[1A\r  ✅ {:<9}: \x1b[1m{:.2} Mbps\x1b[0m ({} em {:.1}s)", 
        label, final_mbps, format_bytes(final_bytes), final_elapsed
    );
    
    Ok((final_mbps, final_bytes))
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

fn format_bytes(bytes: usize) -> String {
    let mb = bytes as f64 / 1_048_576.0;
    if mb > 1024.0 { format!("{:.2} GB", mb / 1024.0) }
    else { format!("{:.2} MB", mb) }
}
