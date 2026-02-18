//! Pulse — A production-grade terminal system monitor with animated cyberpunk TUI.
//!
//! # Architecture
//! - `app`      — Application state, input handling, data coordination
//! - `config`   — TOML configuration system (~/.config/pulse/config.toml)
//! - `event`    — Async event loop (keyboard + periodic tick)
//! - `history`  — Ring-buffer historical metrics engine with JSON export
//! - `plugin`   — Dynamic plugin loading system
//! - `remote`   — SSH-based remote host monitoring
//! - `server`   — Headless JSON server mode for remote monitoring
//! - `system/`  — System data collection (CPU, memory, disk, network, process, GPU)
//! - `ui/`      — All ratatui rendering (layout, panels, animation, theme, CRT effects)
//! - `utils`    — Shared formatting and helper functions

mod app;
mod config;
mod event;
mod history;
mod plugin;
mod remote;
mod server;
mod system;
mod ui;
mod utils;

use std::io;

use clap::Parser;
use color_eyre::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

use app::App;
use config::Config;
use event::{Event, EventLoop};

// ── CLI arguments ────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "pulse", version, about = "A production-grade terminal system monitor")]
struct Cli {
    /// Run in headless server mode (emit JSON lines to stdout for remote monitoring).
    #[arg(long)]
    server: bool,

    /// Override the refresh interval in milliseconds (server mode).
    #[arg(long, default_value_t = 500)]
    interval: u64,
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    // Load configuration
    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config: {e} — using defaults");
        Config::default()
    });

    // Save default config if it doesn't exist
    if !Config::path().exists() {
        let _ = config.save();
    }

    // Server mode: headless JSON output, no TUI
    if cli.server {
        return server::run_server(cli.interval).map_err(Into::into);
    }

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Run the application
    let result = run(&mut terminal, config).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

/// Main application loop. Drives the event loop and rendering pipeline.
async fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, config: Config) -> Result<()> {
    let tick_ms = config.frame_tick_ms();
    let mut app = App::new(config);
    let event_loop = EventLoop::new(tick_ms);

    loop {
        // ── Render ───────────────────────────────────────────────────────
        terminal.draw(|frame| ui::render(frame, &app))?;

        // ── Handle events ────────────────────────────────────────────────
        match event_loop.next().await? {
            Event::Tick => {
                app.on_tick();
            }
            Event::Key(key) => {
                if app.on_key(key) {
                    break; // quit requested
                }
            }
        }
    }

    Ok(())
}
