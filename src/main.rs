//! Pulse — A high-performance terminal system monitor with cyberpunk aesthetics.
//!
//! # Architecture
//! - `app`    — Application state, tick logic, sorting modes
//! - `system` — System data collection via `sysinfo`
//! - `ui`     — All ratatui rendering code
//! - `event`  — Async event loop (keyboard + periodic tick)
//! - `theme`  — Neon cyberpunk color palette

mod app;
mod event;
mod system;
mod theme;
mod ui;

use std::io;

use color_eyre::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

use app::App;
use event::{Event, EventLoop};

// ── entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Run the application
    let result = run(&mut terminal).await;

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
async fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = App::new();
    let event_loop = EventLoop::new(16); // ~60 FPS tick rate (16 ms)

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
