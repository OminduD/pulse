//! UI rendering. Every widget is a pure function of [`App`] state.
//! Nothing here mutates application data.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Gauge, Paragraph, Row, Sparkline, Table, Wrap,
    },
    Frame,
};

use crate::app::App;
use crate::theme;

// ── Top-level layout ─────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Fill entire background
    let bg_block = Block::default().style(Style::default().bg(theme::BG_DARK));
    frame.render_widget(bg_block, size);

    // Main vertical splits: header ─ top row ─ bottom row ─ footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(12),   // top section (CPU + Memory/Disk)
            Constraint::Min(10),   // bottom section (Net + Processes)
            Constraint::Length(1), // footer
        ])
        .split(size);

    draw_header(frame, main_chunks[0], app);
    draw_top_section(frame, main_chunks[1], app);
    draw_bottom_section(frame, main_chunks[2], app);
    draw_footer(frame, main_chunks[3], app);
}

// ── Header with pulsing neon animation ───────────────────────────────────────

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    // Pulse between cyan and purple using the phase
    let pulse = (app.phase * std::f64::consts::TAU).sin() * 0.5 + 0.5;
    let header_color = theme::gradient_color(pulse);

    let title_text = " ⚡ PULSE — SYSTEM MONITOR ⚡ ";

    // Build a line of decorative glyphs that scroll
    let bar_width = area.width as usize;
    let pattern: Vec<char> = "░▒▓█▓▒░".chars().collect();
    let offset = (app.tick_count as usize) % pattern.len();

    let scrolling: String = (0..bar_width)
        .map(|i| pattern[(i + offset) % pattern.len()])
        .collect();

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            scrolling.clone(),
            Style::default().fg(header_color).add_modifier(Modifier::DIM),
        )),
        Line::from(Span::styled(
            format!("{:^width$}", title_text, width = bar_width),
            Style::default()
                .fg(header_color)
                .add_modifier(Modifier::BOLD),
        )),
    ])
    .style(Style::default().bg(theme::BG_PANEL));

    frame.render_widget(header, area);
}

// ── Top section: CPU chart | Memory & Disk ───────────────────────────────────

fn draw_top_section(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    draw_cpu_chart(frame, chunks[0], app);
    draw_memory_disk(frame, chunks[1], app);
}

fn draw_cpu_chart(frame: &mut Frame, area: Rect, app: &App) {
    let block = styled_block(" CPU Usage ");

    // Prepare sparkline data (last N points that fit the widget width)
    let inner = block.inner(area);
    let width = inner.width as usize;

    // We'll render a dual view: sparkline on top, per-core bars below
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(4)])
        .split(inner);

    frame.render_widget(block, area);

    // ── Scrolling sparkline ──────────────────────────────────────────────
    let data: Vec<u64> = app
        .cpu_history
        .iter()
        .rev()
        .take(width)
        .rev()
        .map(|v| *v as u64)
        .collect();

    let color = theme::gradient_color(app.cpu.global / 100.0);

    let sparkline = Sparkline::default()
        .data(&data)
        .max(100)
        .style(Style::default().fg(color));

    frame.render_widget(sparkline, chunks[0]);

    // ── Per-core mini bars ───────────────────────────────────────────────
    let cores = &app.cpu.per_core;
    let max_show = (chunks[1].width as usize / 6).min(cores.len());
    let bar_spans: Vec<Span> = cores
        .iter()
        .take(max_show)
        .enumerate()
        .flat_map(|(i, &usage)| {
            let color = theme::gradient_color(usage / 100.0);
            let bar_char = bar_glyph(usage / 100.0);
            vec![
                Span::styled(
                    format!("{:>2}", i),
                    Style::default().fg(theme::TEXT_DIM),
                ),
                Span::styled(
                    format!("{} ", bar_char),
                    Style::default().fg(color),
                ),
            ]
        })
        .collect();

    let cores_line = Paragraph::new(vec![
        Line::from(Span::styled(
            format!(" Global: {:.1}%", app.cpu.global),
            Style::default()
                .fg(theme::NEON_GREEN)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(bar_spans),
    ]);
    frame.render_widget(cores_line, chunks[1]);
}

/// Return a Unicode block character for 0.0–1.0 ratio.
fn bar_glyph(ratio: f64) -> &'static str {
    match (ratio * 8.0) as u8 {
        0 => " ",
        1 => "▁",
        2 => "▂",
        3 => "▃",
        4 => "▄",
        5 => "▅",
        6 => "▆",
        7 => "▇",
        _ => "█",
    }
}

fn draw_memory_disk(frame: &mut Frame, area: Rect, app: &App) {
    let block = styled_block(" Memory & Disk ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // RAM gauge
            Constraint::Length(3), // Swap gauge
            Constraint::Min(2),   // Disk list
        ])
        .split(inner);

    // ── RAM gauge ────────────────────────────────────────────────────────
    let mem = &app.memory;
    let mem_ratio = if mem.total > 0 {
        mem.used as f64 / mem.total as f64
    } else {
        0.0
    };
    let mem_label = format!(
        "RAM: {:.1} / {:.1} GB ({:.0}%)",
        mem.used as f64 / 1_073_741_824.0,
        mem.total as f64 / 1_073_741_824.0,
        mem_ratio * 100.0
    );
    // Animate the gauge fill with a slight pulse
    let pulse = ((app.phase * std::f64::consts::TAU * 2.0).sin() * 0.02 + 1.0).min(1.0);
    let display_ratio = (mem_ratio * pulse).clamp(0.0, 1.0);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(
            Style::default()
                .fg(theme::gradient_color(mem_ratio))
                .bg(theme::BG_PANEL),
        )
        .ratio(display_ratio)
        .label(Span::styled(
            mem_label,
            Style::default()
                .fg(theme::TEXT_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, chunks[0]);

    // ── Swap gauge ───────────────────────────────────────────────────────
    let swap_ratio = if mem.swap_total > 0 {
        mem.swap_used as f64 / mem.swap_total as f64
    } else {
        0.0
    };
    let swap_label = format!(
        "SWP: {:.1} / {:.1} GB ({:.0}%)",
        mem.swap_used as f64 / 1_073_741_824.0,
        mem.swap_total as f64 / 1_073_741_824.0,
        swap_ratio * 100.0
    );
    let swap_gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(
            Style::default()
                .fg(theme::NEON_PURPLE)
                .bg(theme::BG_PANEL),
        )
        .ratio(swap_ratio.clamp(0.0, 1.0))
        .label(Span::styled(
            swap_label,
            Style::default()
                .fg(theme::TEXT_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(swap_gauge, chunks[1]);

    // ── Disk list ────────────────────────────────────────────────────────
    let disk_lines: Vec<Line> = app
        .disks
        .iter()
        .map(|d| {
            let ratio = if d.total > 0 {
                d.used as f64 / d.total as f64
            } else {
                0.0
            };
            let color = theme::gradient_color(ratio);
            Line::from(vec![
                Span::styled(
                    format!(" {} ", d.name),
                    Style::default().fg(theme::NEON_YELLOW),
                ),
                Span::styled(
                    format!("{} ", d.mount),
                    Style::default().fg(theme::NEON_CYAN),
                ),
                Span::styled(
                    format!("({}) ", d.fs),
                    Style::default().fg(theme::TEXT_DIM),
                ),
                Span::styled(
                    format!(
                        "{:.1}/{:.1}G ",
                        d.used as f64 / 1_073_741_824.0,
                        d.total as f64 / 1_073_741_824.0
                    ),
                    Style::default().fg(color),
                ),
                Span::styled(
                    format!("[{}]", mini_bar(ratio, 10)),
                    Style::default().fg(color),
                ),
            ])
        })
        .collect();

    let disk_para = Paragraph::new(disk_lines).wrap(Wrap { trim: true });
    frame.render_widget(disk_para, chunks[2]);
}

/// Build a small ASCII bar: ████░░░░░░
fn mini_bar(ratio: f64, width: usize) -> String {
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

// ── Bottom section: Network | Process table ──────────────────────────────────

fn draw_bottom_section(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_network(frame, chunks[0], app);
    draw_processes(frame, chunks[1], app);
}

fn draw_network(frame: &mut Frame, area: Rect, app: &App) {
    let block = styled_block(" Network ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(inner);

    // Speed readouts + totals
    let speed_text = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" ▼ RX: ", Style::default().fg(theme::NEON_GREEN)),
            Span::styled(
                format_bytes_speed(app.net.rx_speed),
                Style::default()
                    .fg(theme::NEON_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  tot: {}", format_bytes_total(app.net.rx_bytes)),
                Style::default().fg(theme::TEXT_DIM),
            ),
        ]),
        Line::from(vec![
            Span::styled(" ▲ TX: ", Style::default().fg(theme::NEON_ORANGE)),
            Span::styled(
                format_bytes_speed(app.net.tx_speed),
                Style::default()
                    .fg(theme::NEON_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  tot: {}", format_bytes_total(app.net.tx_bytes)),
                Style::default().fg(theme::TEXT_DIM),
            ),
        ]),
    ]);
    frame.render_widget(speed_text, chunks[0]);

    // RX sparkline
    let width = chunks[1].width as usize;
    let rx_data: Vec<u64> = app
        .net_rx_history
        .iter()
        .rev()
        .take(width)
        .rev()
        .map(|v| (*v / 1024.0) as u64) // KB scale
        .collect();

    let sparkline = Sparkline::default()
        .data(&rx_data)
        .style(Style::default().fg(theme::NEON_GREEN));
    frame.render_widget(sparkline, chunks[1]);
}

fn draw_processes(frame: &mut Frame, area: Rect, app: &App) {
    let sort_label = app.sort_mode.label();
    let title = format!(" Processes [sort: {}] ", sort_label);
    let block = styled_block(&title);

    let header_cells = ["PID", "Name", "CPU%", "MEM(MB)", "Status"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(theme::NEON_CYAN)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells)
        .height(1)
        .style(Style::default().bg(theme::BG_PANEL));

    let rows: Vec<Row> = app
        .processes
        .iter()
        .skip(app.process_scroll)
        .take(area.height.saturating_sub(4) as usize)
        .map(|p| {
            let cpu_color = if p.cpu > 80.0 {
                theme::NEON_RED
            } else {
                theme::gradient_color(p.cpu as f64 / 100.0)
            };
            Row::new(vec![
                Cell::from(format!("{}", p.pid)).style(Style::default().fg(theme::TEXT_DIM)),
                Cell::from(truncate_str(&p.name, 20))
                    .style(Style::default().fg(theme::TEXT_BRIGHT)),
                Cell::from(format!("{:.1}", p.cpu)).style(Style::default().fg(cpu_color)),
                Cell::from(format!("{:.1}", p.mem_mb))
                    .style(Style::default().fg(theme::NEON_PURPLE)),
                Cell::from(p.status.clone()).style(Style::default().fg(theme::TEXT_DIM)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(block)
    .row_highlight_style(theme::highlight_style());

    frame.render_widget(table, area);
}

// ── Footer ───────────────────────────────────────────────────────────────────

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let hints = Line::from(vec![
        Span::styled(" q", Style::default().fg(theme::NEON_PINK).add_modifier(Modifier::BOLD)),
        Span::styled(" Quit  ", Style::default().fg(theme::TEXT_DIM)),
        Span::styled("s", Style::default().fg(theme::NEON_PINK).add_modifier(Modifier::BOLD)),
        Span::styled(" Sort  ", Style::default().fg(theme::TEXT_DIM)),
        Span::styled("↑↓", Style::default().fg(theme::NEON_PINK).add_modifier(Modifier::BOLD)),
        Span::styled(" Scroll  ", Style::default().fg(theme::TEXT_DIM)),
        Span::styled("PgUp/PgDn", Style::default().fg(theme::NEON_PINK).add_modifier(Modifier::BOLD)),
        Span::styled(" Page  ", Style::default().fg(theme::TEXT_DIM)),
        Span::raw("  "),
        Span::styled(
            format!("tick #{}", app.tick_count),
            Style::default().fg(theme::TEXT_DIM),
        ),
    ]);
    let footer = Paragraph::new(hints).style(Style::default().bg(theme::BG_PANEL));
    frame.render_widget(footer, area);
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn styled_block(title: &str) -> Block<'_> {
    Block::default()
        .title(Span::styled(title, theme::title_style()))
        .borders(Borders::ALL)
        .border_style(theme::border_style())
        .style(Style::default().bg(theme::BG_DARK))
}

fn format_bytes_speed(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_073_741_824.0 {
        format!("{:.2} GB/s", bytes_per_sec / 1_073_741_824.0)
    } else if bytes_per_sec >= 1_048_576.0 {
        format!("{:.2} MB/s", bytes_per_sec / 1_048_576.0)
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

fn format_bytes_total(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
