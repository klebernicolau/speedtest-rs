use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Speedtest Auditor - Engine 64KB")]
pub struct Cli {
    #[arg(short = 'd', long = "duration", default_value = "10")]
    pub duration: u64,
    #[arg(long)]
    pub tui: bool,
    /// Salvar resultado em arquivo (ex: --output relatorio.json ou relatorio.csv)
    #[arg(short = 'o', long = "output")]
    pub output: Option<String>,
}
