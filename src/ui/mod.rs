//! UI module. Dispatches rendering to the appropriate layout and panels
//! based on the current application view mode.

pub mod animation;
pub mod crt;
pub mod layout;
pub mod panels;
pub mod theme;

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::app::App;
use layout::{ActiveView, LayoutMode};

/// Top-level render dispatcher. Called every frame.
pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let theme = &app.theme;

    // Fill entire background
    let bg = Block::default().style(Style::default().bg(theme.bg_dark));
    frame.render_widget(bg, size);

    // Matrix rain background (before widgets so panels paint over it)
    if app.config.display.matrix_bg && app.config.display.animations {
        render_matrix_bg(frame, size, app);
    }

    // Show startup splash until it expires or is dismissed
    if app.splash_remaining > 0 {
        panels::draw_splash(frame, size, app);
        return;
    }

    // Render subtle animated wave pattern at the bottom for atmosphere
    if app.config.display.animations {
        render_ambient_effect(frame, size, app);
    }

    match app.layout_mode {
        LayoutMode::Detailed => render_detailed(frame, app, size),
        LayoutMode::Compact => render_compact(frame, app, size),
        LayoutMode::ProcessOnly => render_process_only(frame, app, size),
        LayoutMode::Focus => render_focus(frame, app, size),
    }

    // Overlays (always on top)
    if app.filter_active {
        panels::draw_filter_input(frame, size, app);
    }
    if app.show_process_detail {
        panels::draw_process_detail(frame, size, app);
    }
    panels::draw_status_message(frame, size, app);

    // CRT post-processing: modify the buffer after all widgets are rendered
    if app.crt_enabled {
        let buf = frame.buffer_mut();
        crt::apply_crt_effects(buf, &app.crt_config, app.tick_count);
    }
}

/// Render a subtle ambient background effect — a dim wave at the very bottom.
fn render_ambient_effect(frame: &mut Frame, area: Rect, app: &App) {
    let tick = app.tick_count;
    let theme = &app.theme;

    // Only if there's room
    if area.height < 8 {
        return;
    }

    let wave_y = area.height.saturating_sub(1);
    let wave_area = Rect::new(area.x, wave_y, area.width, 1);
    let wave = animation::wave_pattern(tick, area.width as usize);

    let (r, g, b) = animation::neon_cycle(app.phase);
    let dim_color = Color::Rgb(r / 5, g / 5, b / 5);

    let wave_line = Paragraph::new(Line::from(Span::styled(
        wave,
        Style::default().fg(dim_color).bg(theme.bg_dark),
    )));
    frame.render_widget(wave_line, wave_area);
}

// ── Detailed layout ──────────────────────────────────────────────────────────

fn render_detailed(frame: &mut Frame, app: &App, size: ratatui::layout::Rect) {
    let areas = layout::compute_detailed(size);

    panels::draw_header(frame, areas.header, app);
    panels::draw_footer(frame, areas.footer, app);

    match app.active_view {
        ActiveView::Overview => {
            panels::draw_cpu(frame, areas.top_left, app);
            panels::draw_memory_disk(frame, areas.top_right, app);
            panels::draw_network(frame, areas.bottom_left, app);
            panels::draw_processes(frame, areas.bottom_right, app);
        }
        ActiveView::Network => {
            panels::draw_cpu(frame, areas.top_left, app);
            panels::draw_memory_disk(frame, areas.top_right, app);
            panels::draw_network_detail(frame, areas.bottom_left, app);
            panels::draw_processes(frame, areas.bottom_right, app);
        }
        ActiveView::Disk => {
            panels::draw_cpu(frame, areas.top_left, app);
            panels::draw_disk_detail(frame, areas.top_right, app);
            panels::draw_network(frame, areas.bottom_left, app);
            panels::draw_processes(frame, areas.bottom_right, app);
        }
        ActiveView::Gpu => {
            panels::draw_cpu(frame, areas.top_left, app);
            panels::draw_gpu(frame, areas.top_right, app);
            panels::draw_network(frame, areas.bottom_left, app);
            panels::draw_processes(frame, areas.bottom_right, app);
        }
        ActiveView::History => {
            panels::draw_history(frame, areas.top_left, app);
            panels::draw_memory_disk(frame, areas.top_right, app);
            panels::draw_network(frame, areas.bottom_left, app);
            panels::draw_processes(frame, areas.bottom_right, app);
        }
        ActiveView::Remote => {
            panels::draw_remote(frame, areas.top_left, app);
            panels::draw_memory_disk(frame, areas.top_right, app);
            panels::draw_network(frame, areas.bottom_left, app);
            panels::draw_processes(frame, areas.bottom_right, app);
        }
        ActiveView::Heatmap => {
            panels::draw_heatmap(frame, areas.top_left, app);
            panels::draw_memory_disk(frame, areas.top_right, app);
            panels::draw_network(frame, areas.bottom_left, app);
            panels::draw_processes(frame, areas.bottom_right, app);
        }
        ActiveView::Alerts => {
            panels::draw_alerts(frame, areas.top_left, app);
            panels::draw_memory_disk(frame, areas.top_right, app);
            panels::draw_network(frame, areas.bottom_left, app);
            panels::draw_processes(frame, areas.bottom_right, app);
        }
        ActiveView::Containers => {
            panels::draw_containers(frame, areas.top_left, app);
            panels::draw_memory_disk(frame, areas.top_right, app);
            panels::draw_network(frame, areas.bottom_left, app);
            panels::draw_processes(frame, areas.bottom_right, app);
        }
        ActiveView::Fans => {
            panels::draw_fans(frame, areas.top_left, app);
            panels::draw_memory_disk(frame, areas.top_right, app);
            panels::draw_network(frame, areas.bottom_left, app);
            panels::draw_processes(frame, areas.bottom_right, app);
        }
    }
}

// ── Compact layout ───────────────────────────────────────────────────────────

fn render_compact(frame: &mut Frame, app: &App, size: ratatui::layout::Rect) {
    let areas = layout::compute_compact(size);

    panels::draw_header(frame, areas.header, app);
    panels::draw_footer(frame, areas.footer, app);
    panels::draw_cpu(frame, areas.top_left, app);
    panels::draw_memory_disk(frame, areas.top_right, app);
    panels::draw_network(frame, areas.bottom_left, app);
    panels::draw_processes(frame, areas.bottom_right, app);
}

// ── Process-only layout ──────────────────────────────────────────────────────

fn render_process_only(frame: &mut Frame, app: &App, size: ratatui::layout::Rect) {
    let areas = layout::compute_process_only(size);

    panels::draw_header(frame, areas.header, app);
    panels::draw_footer(frame, areas.footer, app);
    panels::draw_processes(frame, areas.bottom_right, app);
}

// ── Focus layout ─────────────────────────────────────────────────────────────

fn render_focus(frame: &mut Frame, app: &App, size: ratatui::layout::Rect) {
    let areas = layout::compute_focus(size);

    panels::draw_header(frame, areas.header, app);
    panels::draw_footer(frame, areas.footer, app);

    match app.active_view {
        ActiveView::Overview => panels::draw_cpu(frame, areas.top_left, app),
        ActiveView::Network => panels::draw_network_detail(frame, areas.top_left, app),
        ActiveView::Disk => panels::draw_disk_detail(frame, areas.top_left, app),
        ActiveView::Gpu => panels::draw_gpu(frame, areas.top_left, app),
        ActiveView::History => panels::draw_history(frame, areas.top_left, app),
        ActiveView::Remote => panels::draw_remote(frame, areas.top_left, app),
        ActiveView::Heatmap => panels::draw_heatmap(frame, areas.top_left, app),
        ActiveView::Alerts => panels::draw_alerts(frame, areas.top_left, app),
        ActiveView::Containers => panels::draw_containers(frame, areas.top_left, app),
        ActiveView::Fans => panels::draw_fans(frame, areas.top_left, app),
    }
}

// ── Matrix rain background ──────────────────────────────────────────────────

/// Render matrix rain directly to the frame buffer before widgets are drawn.
/// Only writes to cells that still contain the background fill (space character),
/// so widgets that render on top will fully overwrite the rain.
fn render_matrix_bg(frame: &mut Frame, area: Rect, app: &App) {
    let tick = app.tick_count;
    let buf = frame.buffer_mut();
    let width = area.width as usize;
    let height = area.height as usize;

    for col in 0..width {
        let chars = animation::matrix_column(height, tick, col);
        // Per-column "drop speed" derived deterministically from column index
        let speed = 1 + (col.wrapping_mul(7).wrapping_add(3)) % 4;
        let head_row = ((tick as usize).wrapping_mul(speed)) / 3 % (height * 2);

        for row in 0..height {
            let x = area.x + col as u16;
            let y = area.y + row as u16;

            // Only draw below the falling "head"
            let dist = if head_row >= row {
                head_row - row
            } else {
                continue;
            };

            if dist > height {
                continue;
            }

            // Brightness fades with distance from head
            let brightness = if dist == 0 {
                200u8
            } else if dist < 6 {
                (100u8).saturating_sub(dist as u8 * 14)
            } else {
                (50usize.saturating_sub(dist * 3)).max(4) as u8
            };

            let cell = &mut buf[(x, y)];
            // Only write into empty (background) cells
            if cell.symbol() == " " {
                cell.set_char(chars[row]);
                cell.set_fg(Color::Rgb(0, brightness, brightness / 3));
            }
        }
    }
}
