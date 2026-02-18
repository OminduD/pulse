//! Async event loop. Produces [`Event`] variants for key presses, ticks,
//! and data refresh signals. Separates render tick from data tick.

use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{self, KeyEvent, KeyEventKind};

/// Events the main loop can react to.
pub enum Event {
    /// A keyboard key was pressed.
    Key(KeyEvent),
    /// A periodic UI render tick elapsed.
    Tick,
}

/// Drives the event stream. Separates render rate from system data refresh.
pub struct EventLoop {
    /// Frame interval (e.g., 16ms for 60 FPS).
    tick_rate: Duration,
}

impl EventLoop {
    pub fn new(tick_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_ms),
        }
    }

    /// Wait for the next event. If no key arrives within one tick period,
    /// emit a Tick event so the UI redraws.
    pub async fn next(&self) -> Result<Event> {
        if event::poll(self.tick_rate)? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    return Ok(Event::Key(key));
                }
            }
        }
        Ok(Event::Tick)
    }
}
