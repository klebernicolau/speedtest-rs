use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::thread;
use std::time::{Duration, Instant};

use crate::ping::strip_path;

const OOKLA_SERVERS_URL: &str = "https://www.speedtest.net/api/js/servers?engine=js";

pub struct OoklaServer {
    pub url: String,
    pub name: String,
    pub sponsor: String,
}

pub fn get_best_server(client: &Client) -> Result<OoklaServer> {
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
