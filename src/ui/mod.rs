//! UI module. Dispatches rendering to the appropriate layout and panels
//! based on the current application view mode.

pub mod animation;
pub mod layout;
pub mod panels;
pub mod theme;

use ratatui::style::Style;
use ratatui::widgets::Block;
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
    panels::draw_status_message(frame, size, app);
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
    }
}
