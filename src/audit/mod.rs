use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;

pub struct AuditRecord {
    pub timestamp: String,
    pub server_sponsor: String,
    pub server_name: String,
    pub ping_avg_ms: f64,
    pub ping_min_ms: f64,
    pub ping_max_ms: f64,
    pub jitter_ms: f64,
    pub dl_mbps: f64,
    pub ul_mbps: f64,
}

/// Timestamp simples sem dependência de chrono
pub fn now_iso() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let secs  = d % 60;
    let mins  = (d / 60) % 60;
    let hours = (d / 3600) % 24;
    let days  = d / 86400;
    format!("epoch+{}d {:02}:{:02}:{:02}Z", days, hours, mins, secs)
}

pub fn save_audit(path: &str, rec: &AuditRecord) -> Result<()> {
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
        let prefix = if trimmed.is_empty() {
            "[".to_string()
        } else {
            format!("{}{}\n", trimmed, sep)
        };
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
