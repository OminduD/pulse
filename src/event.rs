//! Async event loop. Produces [`Event`] variants for key presses and ticks.
//! Uses crossterm's async event stream under tokio.

use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{self, KeyEvent, KeyEventKind};

/// Events the main loop can react to.
pub enum Event {
    /// A keyboard key was pressed.
    Key(KeyEvent),
    /// A periodic UI tick elapsed.
    Tick,
}

/// Drives the event stream. `tick_ms` controls the UI refresh interval.
pub struct EventLoop {
    tick_rate: Duration,
}

impl EventLoop {
    pub fn new(tick_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_ms),
        }
    }

    /// Wait for the next event (key or tick). Non-blocking thanks to
    /// `crossterm::event::poll`.
    pub async fn next(&self) -> Result<Event> {
        // We use `poll` with the tick duration so that if no key arrives
        // within one tick period we emit a Tick event and let the UI redraw.
        if event::poll(self.tick_rate)? {
            if let event::Event::Key(key) = event::read()? {
                // Only react to key *press* events (ignore release/repeat)
                if key.kind == KeyEventKind::Press {
                    return Ok(Event::Key(key));
                }
            }
        }
        Ok(Event::Tick)
    }
}
