use anyhow::Result;
use clap::Parser;
use reqwest::blocking::Client;
use std::time::Duration;

mod audit;
mod cli;
mod ping;
mod server;
mod speedtest;
mod tui;

use audit::{AuditRecord, now_iso, save_audit};
use cli::Cli;
use ping::measure_ping_full;
use server::get_best_server;
use speedtest::run_cli_test;
use tui::run_tui_mode;

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
        println!("\n==================================================");
        println!(" Servidor : {} ({})", server.sponsor, server.name);
        println!("--------------------------------------------------");
        println!(" Ping Avg : {:.1} ms", ping_result.avg_ms);
        println!(" Ping Min : {:.1} ms", ping_result.min_ms);
        println!(" Ping Max : {:.1} ms", ping_result.max_ms);
        println!(" Jitter   : {:.1} ms", ping_result.jitter_ms);
        println!("==================================================");

        println!();
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

