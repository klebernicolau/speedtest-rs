use anyhow::Result;
use crossterm::{
    execute,
    event::{self, Event, KeyCode},
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph},
    Terminal,
};
use reqwest::blocking::Client;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::server::OoklaServer;
use crate::speedtest::run_tui_test;

pub fn run_tui_mode(
    client: &Client,
    server: &OoklaServer,
    ping: f64,
    jitter: f64,
    duration: u64,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let dl_mbps  = Arc::new(AtomicU64::new(0));
    let ul_mbps  = Arc::new(AtomicU64::new(0));
    let dl_prog  = Arc::new(AtomicU64::new(0));
    let ul_prog  = Arc::new(AtomicU64::new(0));
    let finished = Arc::new(AtomicBool::new(false));

    let (t_dl, t_ul, t_dp, t_up, t_f) = (
        Arc::clone(&dl_mbps),
        Arc::clone(&ul_mbps),
        Arc::clone(&dl_prog),
        Arc::clone(&ul_prog),
        Arc::clone(&finished),
    );
    let c_thread = client.clone();
    let s_url = server.url.clone();

    thread::spawn(move || {
        let _ = run_tui_test(&c_thread, &s_url, duration, true,  t_dl, t_dp);
        let _ = run_tui_test(&c_thread, &s_url, duration, false, t_ul, t_up);
        t_f.store(true, Ordering::SeqCst);
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
                    Constraint::Min(0),
                ].as_ref())
                .split(f.size());

            f.render_widget(
                Paragraph::new(format!(
                    " Servidor: {} | Ping: {:.1}ms | Jitter: {:.1}ms",
                    server.sponsor, ping, jitter
                ))
                .block(Block::default().borders(Borders::ALL).title(" Speedtest Auditor ")),
                chunks[0],
            );

            f.render_widget(
                Gauge::default()
                    .block(Block::default().borders(Borders::ALL).title(format!(
                        " 📥 Download: {:.2} Mbps ",
                        dl_mbps.load(Ordering::Relaxed) as f64 / 100.0
                    )))
                    .gauge_style(Style::default().fg(Color::Cyan))
                    .ratio((dl_prog.load(Ordering::Relaxed) as f64 / 100.0).min(1.0)),
                chunks[1],
            );

            f.render_widget(
                Gauge::default()
                    .block(Block::default().borders(Borders::ALL).title(format!(
                        " 📤 Upload: {:.2} Mbps ",
                        ul_mbps.load(Ordering::Relaxed) as f64 / 100.0
                    )))
                    .gauge_style(Style::default().fg(Color::Green))
                    .ratio((ul_prog.load(Ordering::Relaxed) as f64 / 100.0).min(1.0)),
                chunks[2],
            );

            let status = if finished.load(Ordering::SeqCst) {
                " [ FINALIZADO ] - Ctrl+C para sair."
            } else {
                " [ EXECUTANDO... ] "
            };
            f.render_widget(
                Paragraph::new(status).style(Style::default().add_modifier(Modifier::BOLD)),
                chunks[3],
            );
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c')
                    && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
