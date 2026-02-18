//! Individual panel rendering functions.
//! Each function is a pure view of [`App`] state — no mutations.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Sparkline, Table, Wrap},
    Frame,
};

use crate::app::App;
use crate::ui::animation;
use crate::ui::theme::Theme;
use crate::utils;

// ══════════════════════════════════════════════════════════════════════════════
//  HEADER
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let pulse = animation::pulse_value(app.phase);
    let header_color = theme.gradient_color(pulse);

    let bar_width = area.width as usize;
    let scrolling = animation::scrolling_pattern(app.tick_count, bar_width);

    let uptime_str = utils::format_uptime(app.uptime);
    let title_text = format!(
        " ⚡ PULSE — SYSTEM MONITOR ⚡  [{}] [{}] up: {}",
        app.layout_mode.label(),
        app.active_view.label(),
        uptime_str
    );

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            scrolling,
            Style::default().fg(header_color).add_modifier(Modifier::DIM),
        )),
        Line::from(Span::styled(
            format!("{:^width$}", title_text, width = bar_width),
            Style::default()
                .fg(header_color)
                .add_modifier(Modifier::BOLD),
        )),
    ])
    .style(Style::default().bg(theme.bg_panel));

    frame.render_widget(header, area);
}

// ══════════════════════════════════════════════════════════════════════════════
//  CPU PANEL
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_cpu(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let block = styled_block(" CPU Usage ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // stats line
            Constraint::Min(4),   // sparkline
            Constraint::Length(4), // per-core bars
        ])
        .split(inner);

    // ── Stats line ───────────────────────────────────────────────────────
    let (l1, l5, l15) = app.cpu.load_avg;
    let temp_str = app
        .cpu
        .temperature
        .map(|t| format!(" {:.0}°C", t))
        .unwrap_or_default();

    let stats = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" Global: {:.1}%", app.cpu.global),
                Style::default()
                    .fg(theme.accent_success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  Load: {:.2} {:.2} {:.2}", l1, l5, l15),
                Style::default().fg(theme.text_dim),
            ),
            Span::styled(temp_str, Style::default().fg(theme.accent_warning)),
        ]),
    ]);
    frame.render_widget(stats, chunks[0]);

    // ── Scrolling sparkline ──────────────────────────────────────────────
    let width = chunks[1].width as usize;
    let history = app.history.windowed_data(&app.history.cpu_global);
    let data: Vec<u64> = history
        .iter()
        .rev()
        .take(width)
        .rev()
        .map(|v| *v as u64)
        .collect();

    let color = theme.gradient_color(app.cpu.global / 100.0);
    let sparkline = Sparkline::default()
        .data(&data)
        .max(100)
        .style(Style::default().fg(color));
    frame.render_widget(sparkline, chunks[1]);

    // ── Per-core mini bars ───────────────────────────────────────────────
    let cores = &app.cpu.per_core;
    let max_show = (chunks[2].width as usize / 6).min(cores.len());
    let bar_spans: Vec<Span> = cores
        .iter()
        .take(max_show)
        .enumerate()
        .flat_map(|(i, &usage)| {
            let color = theme.gradient_color(usage / 100.0);
            let bar_char = utils::bar_glyph(usage / 100.0);
            vec![
                Span::styled(
                    format!("{:>2}", i),
                    Style::default().fg(theme.text_dim),
                ),
                Span::styled(format!("{} ", bar_char), Style::default().fg(color)),
            ]
        })
        .collect();

    let freq_str = if let Some(&f) = app.cpu.frequencies.first() {
        format!("  {}MHz", f)
    } else {
        String::new()
    };

    let cores_para = Paragraph::new(vec![
        Line::from(bar_spans),
        Line::from(Span::styled(
            format!(" Cores: {}{}", cores.len(), freq_str),
            Style::default().fg(theme.text_dim),
        )),
    ]);
    frame.render_widget(cores_para, chunks[2]);
}

// ══════════════════════════════════════════════════════════════════════════════
//  MEMORY & DISK PANEL
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_memory_disk(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let block = styled_block(" Memory & Disk ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // RAM gauge
            Constraint::Length(3), // Swap gauge
            Constraint::Length(2), // Cache/Buffer info
            Constraint::Min(2),   // Disk list
        ])
        .split(inner);

    let mem = &app.memory;

    // ── RAM gauge ────────────────────────────────────────────────────────
    let mem_ratio = mem.usage_ratio();
    let mem_label = format!(
        "RAM: {:.1} / {:.1} GB ({:.0}%)",
        utils::bytes_to_gib(mem.used),
        utils::bytes_to_gib(mem.total),
        mem_ratio * 100.0
    );

    let pulse_mod = animation::fast_pulse(app.phase) * 0.02 + 0.98;
    let display_ratio = (mem_ratio * pulse_mod).clamp(0.0, 1.0);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(
            Style::default()
                .fg(theme.gradient_color(mem_ratio))
                .bg(theme.bg_panel),
        )
        .ratio(display_ratio)
        .label(Span::styled(
            mem_label,
            Style::default()
                .fg(theme.text_bright)
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, chunks[0]);

    // ── Swap gauge ───────────────────────────────────────────────────────
    let swap_ratio = mem.swap_ratio();
    let swap_label = format!(
        "SWP: {:.1} / {:.1} GB ({:.0}%)",
        utils::bytes_to_gib(mem.swap_used),
        utils::bytes_to_gib(mem.swap_total),
        swap_ratio * 100.0
    );
    let swap_gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(
            Style::default()
                .fg(theme.accent_secondary)
                .bg(theme.bg_panel),
        )
        .ratio(swap_ratio.clamp(0.0, 1.0))
        .label(Span::styled(
            swap_label,
            Style::default()
                .fg(theme.text_bright)
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(swap_gauge, chunks[1]);

    // ── Cache/Buffers info ───────────────────────────────────────────────
    let cache_info = Paragraph::new(Line::from(vec![
        Span::styled(" Cache: ", Style::default().fg(theme.text_dim)),
        Span::styled(
            utils::format_bytes_total(mem.cached),
            Style::default().fg(theme.accent_primary),
        ),
        Span::styled("  Buf: ", Style::default().fg(theme.text_dim)),
        Span::styled(
            utils::format_bytes_total(mem.buffers),
            Style::default().fg(theme.accent_primary),
        ),
    ]));
    frame.render_widget(cache_info, chunks[2]);

    // ── Disk list ────────────────────────────────────────────────────────
    let disk_lines: Vec<Line> = app
        .disks
        .iter()
        .map(|d| {
            let ratio = d.usage_ratio();
            let color = theme.gradient_color(ratio);
            Line::from(vec![
                Span::styled(
                    format!(" {} ", d.mount),
                    Style::default().fg(theme.accent_primary),
                ),
                Span::styled(
                    format!("({}) ", d.fs),
                    Style::default().fg(theme.text_dim),
                ),
                Span::styled(
                    format!(
                        "{:.1}/{:.1}G ",
                        utils::bytes_to_gib(d.used),
                        utils::bytes_to_gib(d.total)
                    ),
                    Style::default().fg(color),
                ),
                Span::styled(
                    format!("[{}]", utils::mini_bar(ratio, 10)),
                    Style::default().fg(color),
                ),
            ])
        })
        .collect();

    let disk_para = Paragraph::new(disk_lines).wrap(Wrap { trim: true });
    frame.render_widget(disk_para, chunks[2 + 1]);
}

// ══════════════════════════════════════════════════════════════════════════════
//  NETWORK PANEL
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_network(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let block = styled_block(" Network ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(inner);

    // Speed readouts
    let speed_text = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" ▼ RX: ", Style::default().fg(theme.accent_success)),
            Span::styled(
                utils::format_bytes_speed(app.net.rx_speed),
                Style::default()
                    .fg(theme.accent_success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  tot: {}", utils::format_bytes_total(app.net.rx_bytes)),
                Style::default().fg(theme.text_dim),
            ),
        ]),
        Line::from(vec![
            Span::styled(" ▲ TX: ", Style::default().fg(theme.accent_warning)),
            Span::styled(
                utils::format_bytes_speed(app.net.tx_speed),
                Style::default()
                    .fg(theme.accent_warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  tot: {}", utils::format_bytes_total(app.net.tx_bytes)),
                Style::default().fg(theme.text_dim),
            ),
        ]),
    ]);
    frame.render_widget(speed_text, chunks[0]);

    // RX sparkline
    let width = chunks[1].width as usize;
    let rx_history = app.history.windowed_data(&app.history.net_rx);
    let rx_data: Vec<u64> = rx_history
        .iter()
        .rev()
        .take(width)
        .rev()
        .map(|v| (*v / 1024.0) as u64)
        .collect();

    let sparkline = Sparkline::default()
        .data(&rx_data)
        .style(Style::default().fg(theme.accent_success));
    frame.render_widget(sparkline, chunks[1]);
}

// ══════════════════════════════════════════════════════════════════════════════
//  NETWORK DETAILED VIEW
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_network_detail(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let block = styled_block(" Network Inspector ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // aggregate stats
            Constraint::Min(4),   // per-interface table
            Constraint::Min(4),   // active connections
        ])
        .split(inner);

    // Aggregate stats with sparkline
    let agg_text = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" ▼ RX: ", Style::default().fg(theme.accent_success)),
            Span::styled(
                utils::format_bytes_speed(app.net.rx_speed),
                Style::default()
                    .fg(theme.accent_success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  ", Style::default().fg(theme.border_dim)),
            Span::styled(" ▲ TX: ", Style::default().fg(theme.accent_warning)),
            Span::styled(
                utils::format_bytes_speed(app.net.tx_speed),
                Style::default()
                    .fg(theme.accent_warning)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(
                    " Total RX: {}  │  Total TX: {}",
                    utils::format_bytes_total(app.net.rx_bytes),
                    utils::format_bytes_total(app.net.tx_bytes)
                ),
                Style::default().fg(theme.text_dim),
            ),
        ]),
    ]);
    frame.render_widget(agg_text, chunks[0]);

    // Per-interface table
    let header_cells = ["Interface", "RX Speed", "TX Speed", "Total RX", "Total TX"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .interfaces
        .iter()
        .map(|iface| {
            Row::new(vec![
                Cell::from(iface.name.clone()).style(Style::default().fg(theme.text_bright)),
                Cell::from(utils::format_bytes_speed(iface.rx_speed))
                    .style(Style::default().fg(theme.accent_success)),
                Cell::from(utils::format_bytes_speed(iface.tx_speed))
                    .style(Style::default().fg(theme.accent_warning)),
                Cell::from(utils::format_bytes_total(iface.rx_bytes))
                    .style(Style::default().fg(theme.text_dim)),
                Cell::from(utils::format_bytes_total(iface.tx_bytes))
                    .style(Style::default().fg(theme.text_dim)),
            ])
        })
        .collect();

    let iface_table = Table::new(
        rows,
        [
            Constraint::Min(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(Span::styled(" Interfaces ", theme.title_style()))
            .borders(Borders::TOP),
    );
    frame.render_widget(iface_table, chunks[1]);

    // Active TCP connections
    let conn_header = Row::new(
        ["Local", "Remote", "State"]
            .iter()
            .map(|h| {
                Cell::from(*h).style(
                    Style::default()
                        .fg(theme.accent_primary)
                        .add_modifier(Modifier::BOLD),
                )
            }),
    )
    .height(1);

    let conn_rows: Vec<Row> = app
        .tcp_connections
        .iter()
        .take(area.height.saturating_sub(10) as usize)
        .map(|c| {
            let state_color = if c.state == "ESTABLISHED" {
                theme.accent_success
            } else if c.state == "LISTEN" {
                theme.accent_primary
            } else {
                theme.text_dim
            };
            Row::new(vec![
                Cell::from(format!("{}:{}", c.local_addr, c.local_port))
                    .style(Style::default().fg(theme.text_bright)),
                Cell::from(format!("{}:{}", c.remote_addr, c.remote_port))
                    .style(Style::default().fg(theme.text_bright)),
                Cell::from(c.state.clone()).style(Style::default().fg(state_color)),
            ])
        })
        .collect();

    let conn_table = Table::new(
        conn_rows,
        [
            Constraint::Min(20),
            Constraint::Min(20),
            Constraint::Length(14),
        ],
    )
    .header(conn_header)
    .block(
        Block::default()
            .title(Span::styled(" TCP Connections ", theme.title_style()))
            .borders(Borders::TOP),
    );
    frame.render_widget(conn_table, chunks[2]);
}

// ══════════════════════════════════════════════════════════════════════════════
//  DISK DETAILED VIEW
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_disk_detail(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let block = styled_block(" Disk & IO Monitor ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // IO stats
            Constraint::Min(4),   // disk list
        ])
        .split(inner);

    // IO throughput stats
    let io = &app.disk_io;
    let io_text = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" Read: ", Style::default().fg(theme.accent_success)),
            Span::styled(
                utils::format_bytes_speed(io.read_speed),
                Style::default()
                    .fg(theme.accent_success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  Write: ", Style::default().fg(theme.accent_warning)),
            Span::styled(
                utils::format_bytes_speed(io.write_speed),
                Style::default()
                    .fg(theme.accent_warning)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" IO Wait: {:.1}%", io.io_wait_pct),
                Style::default().fg(if io.io_wait_pct > 20.0 {
                    theme.accent_error
                } else {
                    theme.text_dim
                }),
            ),
            Span::styled(
                format!(
                    "  │  Total R: {} W: {}",
                    utils::format_bytes_total(io.total_read),
                    utils::format_bytes_total(io.total_write)
                ),
                Style::default().fg(theme.text_dim),
            ),
        ]),
    ]);
    frame.render_widget(io_text, chunks[0]);

    // Disk usage table
    let header_cells = ["Mount", "FS", "Used", "Total", "Usage"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .disks
        .iter()
        .map(|d| {
            let ratio = d.usage_ratio();
            let color = theme.gradient_color(ratio);
            Row::new(vec![
                Cell::from(d.mount.clone()).style(Style::default().fg(theme.text_bright)),
                Cell::from(d.fs.clone()).style(Style::default().fg(theme.text_dim)),
                Cell::from(format!("{:.1}G", utils::bytes_to_gib(d.used)))
                    .style(Style::default().fg(color)),
                Cell::from(format!("{:.1}G", utils::bytes_to_gib(d.total)))
                    .style(Style::default().fg(theme.text_dim)),
                Cell::from(format!("[{}] {:.0}%", utils::mini_bar(ratio, 10), ratio * 100.0))
                    .style(Style::default().fg(color)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(15),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(18),
        ],
    )
    .header(header);
    frame.render_widget(table, chunks[1]);
}

// ══════════════════════════════════════════════════════════════════════════════
//  GPU PANEL
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_gpu(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let block = styled_block(" GPU Monitor ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if !app.gpu.available {
        let msg = Paragraph::new(Line::from(Span::styled(
            " No GPU detected (NVIDIA/AMD)",
            Style::default().fg(theme.text_dim),
        )));
        frame.render_widget(msg, inner);
        return;
    }

    let mut lines = Vec::new();
    for gpu in &app.gpu.gpus {
        let mem_ratio = if gpu.mem_total_mib > 0 {
            gpu.mem_used_mib as f64 / gpu.mem_total_mib as f64
        } else {
            0.0
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", gpu.name),
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                format!("  Usage: {:.0}%", gpu.usage_pct),
                Style::default().fg(theme.gradient_color(gpu.usage_pct as f64 / 100.0)),
            ),
            Span::styled(
                format!(
                    "  VRAM: {}/{}MiB [{:.0}%]",
                    gpu.mem_used_mib,
                    gpu.mem_total_mib,
                    mem_ratio * 100.0
                ),
                Style::default().fg(theme.accent_secondary),
            ),
        ]));

        let mut detail_spans = Vec::new();
        if let Some(temp) = gpu.temperature {
            detail_spans.push(Span::styled(
                format!("  Temp: {:.0}°C", temp),
                Style::default().fg(if temp > 80.0 {
                    theme.accent_error
                } else {
                    theme.accent_warning
                }),
            ));
        }
        if let Some(power) = gpu.power_watts {
            detail_spans.push(Span::styled(
                format!("  Power: {:.0}W", power),
                Style::default().fg(theme.text_dim),
            ));
        }
        if let Some(fan) = gpu.fan_pct {
            detail_spans.push(Span::styled(
                format!("  Fan: {:.0}%", fan),
                Style::default().fg(theme.text_dim),
            ));
        }
        if !detail_spans.is_empty() {
            lines.push(Line::from(detail_spans));
        }
    }

    let gpu_para = Paragraph::new(lines);
    frame.render_widget(gpu_para, inner);
}

// ══════════════════════════════════════════════════════════════════════════════
//  HISTORY VIEW
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_history(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let title = format!(" History [{}] ", app.history.window.label());
    let block = styled_block(&title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(inner);

    // CPU history sparkline
    draw_history_sparkline(frame, chunks[0], app, "CPU %", &app.history.cpu_global, 100, theme.accent_success);
    // Memory history
    draw_history_sparkline(frame, chunks[1], app, "MEM %", &app.history.memory_ratio, 1, theme.accent_secondary);
    // Net RX
    draw_history_sparkline(frame, chunks[2], app, "Net RX", &app.history.net_rx, 0, theme.accent_primary);
    // Disk Read
    draw_history_sparkline(frame, chunks[3], app, "Disk R", &app.history.disk_read, 0, theme.accent_warning);
}

fn draw_history_sparkline(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    label: &str,
    ring: &crate::history::RingBuffer,
    max_val: u64,
    color: ratatui::style::Color,
) {
    let theme = &app.theme;
    let width = area.width as usize;
    let data_vec = app.history.windowed_data(ring);

    // For ratio-based metrics (0.0–1.0), scale to percentage
    let scale = if max_val == 1 { 100.0 } else { 1.0 };

    let data: Vec<u64> = data_vec
        .iter()
        .rev()
        .take(width)
        .rev()
        .map(|v| (*v * scale) as u64)
        .collect();

    let max = if max_val == 0 {
        data.iter().copied().max().unwrap_or(1).max(1)
    } else if max_val == 1 {
        100
    } else {
        max_val
    };

    let latest = data.last().copied().unwrap_or(0);
    let block = Block::default()
        .title(Span::styled(
            format!(" {} ({}) ", label, latest),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme.border_dim));

    let sparkline = Sparkline::default()
        .block(block)
        .data(&data)
        .max(max)
        .style(Style::default().fg(color));
    frame.render_widget(sparkline, area);
}

// ══════════════════════════════════════════════════════════════════════════════
//  PROCESS TABLE
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_processes(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let sort_label = app.sort_mode.label();
    let filter_info = if app.filter_active {
        format!(" filter: \"{}\"", app.filter_text)
    } else {
        String::new()
    };
    let title = format!(" Processes [sort: {}]{} ", sort_label, filter_info);
    let block = styled_block(&title, theme);

    let header_cells = ["PID", "User", "Name", "CPU%", "MEM(MB)", "Thr", "Nice", "Status"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells)
        .height(1)
        .style(Style::default().bg(theme.bg_panel));

    let visible_procs = app.filtered_processes();
    let visible_height = area.height.saturating_sub(4) as usize;

    let rows: Vec<Row> = visible_procs
        .iter()
        .skip(app.process_scroll)
        .take(visible_height)
        .map(|p| {
            let cpu_color = if p.anomaly.high_cpu {
                theme.accent_error
            } else {
                theme.gradient_color(p.cpu as f64 / 100.0)
            };

            let name_style = if p.anomaly.suspicious {
                Style::default()
                    .fg(theme.accent_error)
                    .add_modifier(Modifier::BOLD)
            } else if p.anomaly.high_memory {
                Style::default().fg(theme.accent_warning)
            } else {
                Style::default().fg(theme.text_bright)
            };

            let threads_str = p
                .threads
                .map(|t| t.to_string())
                .unwrap_or_else(|| "-".to_string());

            Row::new(vec![
                Cell::from(format!("{}", p.pid)).style(Style::default().fg(theme.text_dim)),
                Cell::from(utils::truncate_str(&p.user, 8))
                    .style(Style::default().fg(theme.text_dim)),
                Cell::from(utils::truncate_str(&p.name, 20)).style(name_style),
                Cell::from(format!("{:.1}", p.cpu)).style(Style::default().fg(cpu_color)),
                Cell::from(format!("{:.1}", p.mem_mb))
                    .style(Style::default().fg(theme.accent_secondary)),
                Cell::from(threads_str).style(Style::default().fg(theme.text_dim)),
                Cell::from(format!("{}", p.nice)).style(Style::default().fg(theme.text_dim)),
                Cell::from(p.status.clone()).style(Style::default().fg(theme.text_dim)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Min(16),
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(9),
        ],
    )
    .header(header)
    .block(block)
    .row_highlight_style(theme.highlight_style());

    frame.render_widget(table, area);
}

// ══════════════════════════════════════════════════════════════════════════════
//  FOOTER
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let key_style = Style::default()
        .fg(theme.accent_tertiary)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme.text_dim);

    let mut hints = vec![
        Span::styled(" q", key_style),
        Span::styled(" Quit ", desc_style),
        Span::styled("s", key_style),
        Span::styled(" Sort ", desc_style),
        Span::styled("f", key_style),
        Span::styled(" Filter ", desc_style),
        Span::styled("k", key_style),
        Span::styled(" Kill ", desc_style),
        Span::styled("m", key_style),
        Span::styled(" Mode ", desc_style),
        Span::styled("t", key_style),
        Span::styled(" Theme ", desc_style),
        Span::styled("n", key_style),
        Span::styled(" Net ", desc_style),
        Span::styled("d", key_style),
        Span::styled(" Disk ", desc_style),
        Span::styled("g", key_style),
        Span::styled(" GPU ", desc_style),
        Span::styled("h", key_style),
        Span::styled(" Hist ", desc_style),
    ];

    if app.config.security.enabled {
        hints.push(Span::styled(" [SEC]", Style::default().fg(theme.accent_error)));
    }

    hints.push(Span::styled(
        format!("  Theme:{}", theme.id.label()),
        desc_style,
    ));

    let footer = Paragraph::new(Line::from(hints)).style(Style::default().bg(theme.bg_panel));
    frame.render_widget(footer, area);
}

// ══════════════════════════════════════════════════════════════════════════════
//  FILTER INPUT OVERLAY
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_filter_input(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    // Draw a centered input box
    let width = 50.min(area.width.saturating_sub(4));
    let height = 3;
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;
    let input_area = Rect::new(x, y, width, height);

    let block = Block::default()
        .title(Span::styled(
            " Filter (regex) ",
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent_primary))
        .style(Style::default().bg(theme.bg_panel));

    let input = Paragraph::new(Line::from(Span::styled(
        format!("{}_", app.filter_text),
        Style::default()
            .fg(theme.text_bright)
            .add_modifier(Modifier::BOLD),
    )))
    .block(block);

    frame.render_widget(input, input_area);
}

// ══════════════════════════════════════════════════════════════════════════════
//  STATUS MESSAGE OVERLAY
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_status_message(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(ref msg) = app.status_message {
        let theme = &app.theme;
        let width = (msg.len() as u16 + 4).min(area.width.saturating_sub(4));
        let x = (area.width.saturating_sub(width)) / 2 + area.x;
        let y = area.height.saturating_sub(3) + area.y;
        let msg_area = Rect::new(x, y, width, 1);

        let para = Paragraph::new(Line::from(Span::styled(
            format!(" {} ", msg),
            Style::default()
                .fg(theme.accent_warning)
                .add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().bg(theme.bg_panel));

        frame.render_widget(para, msg_area);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
//  HELPERS
// ══════════════════════════════════════════════════════════════════════════════

fn styled_block<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    Block::default()
        .title(Span::styled(title, theme.title_style()))
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .style(Style::default().bg(theme.bg_dark))
}
