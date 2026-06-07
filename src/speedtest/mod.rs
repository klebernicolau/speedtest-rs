use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::ping::strip_path;

pub const DOWNLOAD_SIZE: usize = 50_000_000;
pub const UPLOAD_CHUNK: usize = 1_048_576;

/// Gera um buffer de bytes pseudo-aleatórios usando xorshift64.
/// Evita que servidores comprimam os dados no upload, garantindo medição real.
pub fn random_payload(size: usize) -> Vec<u8> {
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

pub fn run_cli_test(client: &Client, base_url: &str, duration: u64, is_dl: bool) -> Result<(f64, usize)> {
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
        let url = if is_dl {
            format!("{}/download?size={}", base, DOWNLOAD_SIZE)
        } else {
            format!("{}/upload", base)
        };

        handles.push(thread::spawn(move || {
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
                        if res.status().is_success() {
                            t.fetch_add(UPLOAD_CHUNK as u64, Ordering::Relaxed);
                        }
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

    let final_mbps = (total_bytes.load(Ordering::Relaxed) as f64 * 8.0
        / start.elapsed().as_secs_f64()) / 1_000_000.0;
    pb.set_message(format!("| {:.2} Mbps [OK]", final_mbps));
    pb.finish();

    Ok((final_mbps, total_bytes.load(Ordering::Relaxed) as usize))
}

pub fn run_tui_test(
    client: &Client,
    base_url: &str,
    duration: u64,
    is_dl: bool,
    mbps_atom: Arc<AtomicU64>,
    prog_atom: Arc<AtomicU64>,
) -> Result<()> {
    let base = strip_path(base_url);
    let total_bytes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..num_cpus::get().min(8) {
        let t = Arc::clone(&total_bytes);
        let r = Arc::clone(&running);
        let c = client.clone();
        let url = if is_dl {
            format!("{}/download?size={}", base, DOWNLOAD_SIZE)
        } else {
            format!("{}/upload", base)
        };

        handles.push(thread::spawn(move || {
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
                        if res.status().is_success() {
                            t.fetch_add(UPLOAD_CHUNK as u64, Ordering::Relaxed);
                        }
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
        prog_atom.store(
            ((elapsed / duration as f64) * 100.0) as u64,
            Ordering::Relaxed,
        );
        thread::sleep(Duration::from_millis(200));
    }

    running.store(false, Ordering::SeqCst);
    for h in handles { let _ = h.join(); }
    prog_atom.store(100, Ordering::SeqCst);
    Ok(())
}
