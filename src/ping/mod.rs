use anyhow::Result;
use reqwest::blocking::Client;
use std::thread;
use std::time::Duration;
use std::time::Instant;

pub struct PingResult {
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub jitter_ms: f64,
}

/// Remove o último segmento de path de uma URL
pub fn strip_path(url: &str) -> String {
    let parts: Vec<&str> = url.rsplitn(2, '/').collect();
    parts[1].to_string()
}

pub fn measure_ping_full(client: &Client, url: &str, count: usize) -> Result<PingResult> {
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
