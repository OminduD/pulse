//! Individual panel rendering functions.
//! Each function is a pure view of [`App`] state — no mutations.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Sparkline, Table, Wrap},
    Frame,
};

use crate::app::{AlertSeverity, App};
use crate::remote::ConnectionStatus;
use crate::system::process::ProcessInfo;
use crate::ui::animation;
use crate::ui::theme::Theme;
use crate::utils;

// ══════════════════════════════════════════════════════════════════════════════
//  HEADER
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let tick = app.tick_count;
    let phase = app.phase;
    let bar_width = area.width as usize;

    // Animated scan line across the top
    let scan = animation::scan_line(tick, bar_width);

    // Rainbow-tinted title based on phase
    let (nr, ng, nb) = animation::neon_cycle(phase);
    let title_color = Color::Rgb(nr, ng, nb);

    let uptime_str = utils::format_uptime(app.uptime);
    let spin = animation::dot_spinner(tick);
    let title_text = format!(
        " {} PULSE — SYSTEM MONITOR {}  [{}] [{}] up: {}",
        spin,
        spin,
        app.layout_mode.label(),
        app.active_view.label(),
        uptime_str,
    );

    // Breathing glow on the border
    let glow = animation::breathing(phase);
    let border_color = crate::ui::theme::lerp_color(
        theme.border_dim,
        theme.header_accent,
        glow,
    );

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            scan,
            Style::default()
                .fg(Color::Rgb(
                    (nr as f64 * 0.4) as u8,
                    (ng as f64 * 0.4) as u8,
                    (nb as f64 * 0.4) as u8,
                ))
                .add_modifier(Modifier::DIM),
        )),
        Line::from(Span::styled(
            format!("{:^width$}", title_text, width = bar_width),
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(border_color))
            .border_set(symbols::border::DOUBLE),
    )
    .style(Style::default().bg(theme.bg_panel));

    frame.render_widget(header, area);
}

// ══════════════════════════════════════════════════════════════════════════════
//  CPU PANEL
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_cpu(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let tick = app.tick_count;
    let phase = app.phase;

    // Animated border that glows based on CPU load
    let cpu_activity = app.cpu.global / 100.0;
    let block = animated_block(" ⚡ CPU Usage ", theme, phase, cpu_activity);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // stats line
            Constraint::Length(2), // activity bar
            Constraint::Min(3),   // sparkline
            Constraint::Length(4), // per-core bars
        ])
        .split(inner);

    // ── Stats line with animated values ──────────────────────────────────
    let (l1, l5, l15) = app.cpu.load_avg;
    let temp_str = app
        .cpu
        .temperature
        .map(|t| format!(" {:.0}°C", t))
        .unwrap_or_default();

    let cpu_color = theme.gradient_color(cpu_activity);
    let spin = animation::spinner(tick);

    let stats = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" {} Global: ", spin),
                Style::default().fg(theme.text_dim),
            ),
            Span::styled(
                format!("{:.1}%", app.cpu.global),
                Style::default()
                    .fg(cpu_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  Load: {:.2} {:.2} {:.2}", l1, l5, l15),
                Style::default().fg(theme.text_dim),
            ),
            Span::styled(
                temp_str,
                Style::default().fg(if app.cpu.temperature.unwrap_or(0.0) > 80.0 {
                    theme.accent_error
                } else {
                    theme.accent_warning
                }),
            ),
        ]),
    ]);
    frame.render_widget(stats, chunks[0]);

    // ── Animated activity bar ────────────────────────────────────────────
    let bar_width = chunks[1].width.saturating_sub(2) as usize;
    let activity_str = animation::activity_indicator(cpu_activity, bar_width, tick);
    let activity_bar = Paragraph::new(Line::from(Span::styled(
        format!(" {}", activity_str),
        Style::default().fg(cpu_color),
    )));
    frame.render_widget(activity_bar, chunks[1]);

    // ── Scrolling sparkline ──────────────────────────────────────────────
    let width = chunks[2].width as usize;
    let history = app.history.windowed_data(&app.history.cpu_global);
    let data: Vec<u64> = history
        .iter()
        .rev()
        .take(width)
        .rev()
        .map(|v| *v as u64)
        .collect();

    let sparkline = Sparkline::default()
        .data(&data)
        .max(100)
        .style(Style::default().fg(cpu_color));
    frame.render_widget(sparkline, chunks[2]);

    // ── Per-core animated bars ───────────────────────────────────────────
    let cores = &app.cpu.per_core;
    let core_bars = animation::core_activity_bars(cores, tick);
    let max_show = (chunks[3].width as usize / 5).min(core_bars.len());

    let bar_spans: Vec<Span> = core_bars
        .iter()
        .take(max_show)
        .enumerate()
        .flat_map(|(i, (bar_char, ratio))| {
            let color = theme.gradient_color(*ratio);
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
            format!(
                " {} Cores: {}{}",
                animation::dot_spinner(tick),
                cores.len(),
                freq_str
            ),
            Style::default().fg(theme.text_dim),
        )),
    ]);
    frame.render_widget(cores_para, chunks[3]);
}

// ══════════════════════════════════════════════════════════════════════════════
//  MEMORY & DISK PANEL
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_memory_disk(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let tick = app.tick_count;
    let phase = app.phase;

    let mem_activity = app.memory.usage_ratio();
    let block = animated_block(" 🧠 Memory & Disk ", theme, phase, mem_activity);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // RAM gauge
            Constraint::Length(3), // Swap gauge
            Constraint::Length(3), // Cache/Buffer info + sparkline
            Constraint::Min(2),   // Disk list
        ])
        .split(inner);

    let mem = &app.memory;

    // ── RAM gauge with shimmer ───────────────────────────────────────────
    let mem_ratio = mem.usage_ratio();
    let bar_w = chunks[0].width.saturating_sub(4) as usize;
    let shimmer = animation::shimmer_bar(mem_ratio, bar_w, tick);
    let mem_color = theme.gradient_color(mem_ratio);

    let mem_label = format!(
        "RAM: {:.1} / {:.1} GB ({:.0}%)",
        utils::bytes_to_gib(mem.used),
        utils::bytes_to_gib(mem.total),
        mem_ratio * 100.0
    );

    let ram_display = Paragraph::new(vec![
        Line::from(Span::styled(
            format!(" {}", mem_label),
            Style::default()
                .fg(theme.text_bright)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(" {}", shimmer),
            Style::default().fg(mem_color),
        )),
    ]);
    frame.render_widget(ram_display, chunks[0]);

    // ── Swap gauge ───────────────────────────────────────────────────────
    let swap_ratio = mem.swap_ratio();
    let swap_shimmer = animation::gradient_bar(swap_ratio, bar_w, tick);
    let swap_label = format!(
        "SWP: {:.1} / {:.1} GB ({:.0}%)",
        utils::bytes_to_gib(mem.swap_used),
        utils::bytes_to_gib(mem.swap_total),
        swap_ratio * 100.0
    );
    let swap_display = Paragraph::new(vec![
        Line::from(Span::styled(
            format!(" {}", swap_label),
            Style::default()
                .fg(theme.text_bright)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(" {}", swap_shimmer),
            Style::default().fg(theme.accent_secondary),
        )),
    ]);
    frame.render_widget(swap_display, chunks[1]);

    // ── Cache/Buffers with braille sparkline ─────────────────────────────
    let mem_history = app.history.windowed_data(&app.history.memory_ratio);
    let mem_data: Vec<f64> = mem_history.iter().rev().take(20).rev().copied().collect();
    let braille = animation::braille_sparkline(&mem_data, 1.0, 20);

    let cache_info = Paragraph::new(vec![
        Line::from(vec![
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
        ]),
        Line::from(vec![
            Span::styled(" Trend: ", Style::default().fg(theme.text_dim)),
            Span::styled(braille, Style::default().fg(theme.accent_secondary)),
        ]),
    ]);
    frame.render_widget(cache_info, chunks[2]);

    // ── Disk list with animated bars ─────────────────────────────────────
    let disk_lines: Vec<Line> = app
        .disks
        .iter()
        .map(|d| {
            let ratio = d.usage_ratio();
            let color = theme.gradient_color(ratio);
            let mini = animation::shimmer_bar(ratio, 10, tick);
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
                Span::styled(format!("[{}]", mini), Style::default().fg(color)),
            ])
        })
        .collect();

    let disk_para = Paragraph::new(disk_lines).wrap(Wrap { trim: true });
    frame.render_widget(disk_para, chunks[3]);
}

// ══════════════════════════════════════════════════════════════════════════════
//  NETWORK PANEL
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_network(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let tick = app.tick_count;
    let phase = app.phase;

    let net_activity = ((app.net.rx_speed + app.net.tx_speed) / 1_048_576.0).clamp(0.0, 1.0);
    let block = animated_block(" 🌐 Network ", theme, phase, net_activity);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Speed readouts with indicators
            Constraint::Min(3),   // RX sparkline
        ])
        .split(inner);

    // ── Speed readouts with animated indicators ──────────────────────────
    let rx_indicator = animation::activity_indicator(
        (app.net.rx_speed / 1_048_576.0).clamp(0.0, 1.0),
        8,
        tick,
    );
    let tx_indicator = animation::activity_indicator(
        (app.net.tx_speed / 1_048_576.0).clamp(0.0, 1.0),
        8,
        tick,
    );

    let speed_text = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" ▼ RX: ", Style::default().fg(theme.accent_success)),
            Span::styled(
                utils::format_bytes_speed(app.net.rx_speed),
                Style::default()
                    .fg(theme.accent_success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {}", rx_indicator), Style::default().fg(theme.accent_success)),
        ]),
        Line::from(vec![
            Span::styled(" ▲ TX: ", Style::default().fg(theme.accent_warning)),
            Span::styled(
                utils::format_bytes_speed(app.net.tx_speed),
                Style::default()
                    .fg(theme.accent_warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {}", tx_indicator), Style::default().fg(theme.accent_warning)),
        ]),
        Line::from(vec![
            Span::styled(
                format!(
                    " {} tot: {}  │  tot: {}",
                    animation::dot_spinner(tick),
                    utils::format_bytes_total(app.net.rx_bytes),
                    utils::format_bytes_total(app.net.tx_bytes)
                ),
                Style::default().fg(theme.text_dim),
            ),
        ]),
    ]);
    frame.render_widget(speed_text, chunks[0]);

    // ── Dual sparklines for RX/TX ────────────────────────────────────────
    let sparkline_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let width = sparkline_chunks[0].width as usize;

    let rx_history = app.history.windowed_data(&app.history.net_rx);
    let rx_data: Vec<u64> = rx_history
        .iter()
        .rev()
        .take(width)
        .rev()
        .map(|v| (*v / 1024.0) as u64)
        .collect();
    let rx_spark = Sparkline::default()
        .data(&rx_data)
        .style(Style::default().fg(theme.accent_success));
    frame.render_widget(rx_spark, sparkline_chunks[0]);

    let tx_history = app.history.windowed_data(&app.history.net_tx);
    let tx_data: Vec<u64> = tx_history
        .iter()
        .rev()
        .take(width)
        .rev()
        .map(|v| (*v / 1024.0) as u64)
        .collect();
    let tx_spark = Sparkline::default()
        .data(&tx_data)
        .style(Style::default().fg(theme.accent_warning));
    frame.render_widget(tx_spark, sparkline_chunks[1]);
}

// ══════════════════════════════════════════════════════════════════════════════
//  NETWORK DETAILED VIEW
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_network_detail(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let tick = app.tick_count;
    let phase = app.phase;
    let net_activity = ((app.net.rx_speed + app.net.tx_speed) / 1_048_576.0).clamp(0.0, 1.0);
    let block = animated_block(" 🌐 Network Inspector ", theme, phase, net_activity);
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
            Span::styled(
                format!(" {} ", animation::dot_spinner(tick)),
                Style::default().fg(theme.accent_primary),
            ),
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
    let phase = app.phase;
    let tick = app.tick_count;
    let io_activity = (app.disk_io.io_wait_pct / 100.0).clamp(0.0, 1.0);
    let block = animated_block(" 💾 Disk & IO Monitor ", theme, phase, io_activity);
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
            Span::styled(
                format!(" {} ", animation::dot_spinner(tick)),
                Style::default().fg(theme.accent_primary),
            ),
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
    let tick = app.tick_count;
    let phase = app.phase;
    let gpu_activity = if app.gpu.available && !app.gpu.gpus.is_empty() {
        app.gpu.gpus[0].usage_pct as f64 / 100.0
    } else {
        0.0
    };
    let block = animated_block(" 🎮 GPU Monitor ", theme, phase, gpu_activity);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if !app.gpu.available {
        let wave = animation::wave_pattern(tick, inner.width as usize);
        let msg = Paragraph::new(vec![
            Line::from(Span::styled(
                " No GPU detected (NVIDIA/AMD)",
                Style::default().fg(theme.text_dim),
            )),
            Line::from(Span::styled(wave, Style::default().fg(theme.border_dim))),
        ]);
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
        let usage_ratio = gpu.usage_pct as f64 / 100.0;
        let usage_bar_w = 20;
        let usage_bar = animation::shimmer_bar(usage_ratio, usage_bar_w, tick);
        let vram_bar = animation::gradient_bar(mem_ratio, usage_bar_w, tick);

        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} {} ", animation::spinner(tick), gpu.name),
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Usage: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{:.0}% ", gpu.usage_pct),
                Style::default().fg(theme.gradient_color(usage_ratio)),
            ),
            Span::styled(
                format!("[{}]", usage_bar),
                Style::default().fg(theme.gradient_color(usage_ratio)),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  VRAM:  ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!(
                    "{}/{}MiB ",
                    gpu.mem_used_mib, gpu.mem_total_mib
                ),
                Style::default().fg(theme.accent_secondary),
            ),
            Span::styled(
                format!("[{}]", vram_bar),
                Style::default().fg(theme.accent_secondary),
            ),
        ]));

        let mut detail_spans = Vec::new();
        if let Some(temp) = gpu.temperature {
            let temp_warn = temp > 80.0;
            detail_spans.push(Span::styled(
                format!(
                    "  Temp: {:.0}°C{}",
                    temp,
                    if temp_warn && animation::flicker(tick, 8) { " ⚠" } else { "" }
                ),
                Style::default().fg(if temp_warn {
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
    let phase = app.phase;
    let title = format!(" 📊 History [{}] ", app.history.window.label());
    let block = animated_block(&title, theme, phase, 0.3);
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
    let tick = app.tick_count;
    let width = area.width as usize;
    let data_vec = app.history.windowed_data(ring);

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
    let spark_char = animation::spinner(tick);
    let block = Block::default()
        .title(Span::styled(
            format!(" {} {} ({}) ", spark_char, label, latest),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::TOP)
        .border_style(theme.glow_border_style(app.phase));

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
    let tick = app.tick_count;
    let phase = app.phase;
    let sort_label = app.sort_mode.label();
    let filter_info = if app.filter_active {
        format!(" filter: \"{}\"", app.filter_text)
    } else {
        String::new()
    };
    let proc_count = app.filtered_processes().len();
    let tree_label = if app.tree_view { " [TREE]" } else { "" };
    let title = format!(
        " {} Processes [{}] sort:{}{}{} ",
        animation::dot_spinner(tick),
        proc_count,
        sort_label,
        filter_info,
        tree_label,
    );
    let block = animated_block(&title, theme, phase, 0.2);

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

    let visible_height = area.height.saturating_sub(4) as usize;

    // Build rows — tree mode or flat mode
    let rows: Vec<Row> = if app.tree_view {
        let tree = app.tree_processes();
        tree.iter()
            .skip(app.process_scroll)
            .take(visible_height)
            .enumerate()
            .map(|(idx, (p, depth))| {
                let indent = if *depth > 0 {
                    format!("{}└─", "  ".repeat(depth.saturating_sub(1)))
                } else {
                    String::new()
                };
                let max_name_len = 20usize.saturating_sub(depth * 2);
                let display_name = format!("{}{}", indent, utils::truncate_str(&p.name, max_name_len));
                build_process_row(p, idx, &display_name, theme, tick)
            })
            .collect()
    } else {
        let visible_procs = app.filtered_processes();
        visible_procs
            .iter()
            .skip(app.process_scroll)
            .take(visible_height)
            .enumerate()
            .map(|(idx, p)| {
                let display_name = utils::truncate_str(&p.name, 20);
                build_process_row(p, idx, &display_name, theme, tick)
            })
            .collect()
    };

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
    .row_highlight_style(
        Style::default()
            .fg(theme.bg_dark)
            .bg(theme.accent_primary)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_widget(table, area);
}

fn build_process_row<'a>(
    p: &ProcessInfo,
    idx: usize,
    display_name: &str,
    theme: &Theme,
    tick: u64,
) -> Row<'a> {
    let cpu_color = if p.anomaly.high_cpu {
        if animation::flicker(tick, 6) {
            theme.accent_error
        } else {
            theme.accent_warning
        }
    } else {
        theme.gradient_color(p.cpu as f64 / 100.0)
    };

    let name_style = if p.anomaly.suspicious {
        Style::default()
            .fg(theme.accent_error)
            .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK)
    } else if p.anomaly.high_memory {
        Style::default().fg(theme.accent_warning)
    } else {
        Style::default().fg(theme.text_bright)
    };

    let row_bg = if idx % 2 == 0 {
        theme.bg_dark
    } else {
        theme.bg_panel
    };

    let threads_str = p
        .threads
        .map(|t| t.to_string())
        .unwrap_or_else(|| "-".to_string());

    Row::new(vec![
        Cell::from(format!("{}", p.pid)).style(Style::default().fg(theme.text_dim)),
        Cell::from(utils::truncate_str(&p.user, 8)).style(Style::default().fg(theme.text_dim)),
        Cell::from(display_name.to_string()).style(name_style),
        Cell::from(format!("{:.1}", p.cpu)).style(Style::default().fg(cpu_color)),
        Cell::from(format!("{:.1}", p.mem_mb)).style(Style::default().fg(theme.accent_secondary)),
        Cell::from(threads_str).style(Style::default().fg(theme.text_dim)),
        Cell::from(format!("{}", p.nice)).style(Style::default().fg(theme.text_dim)),
        Cell::from(p.status.clone()).style(Style::default().fg(theme.text_dim)),
    ])
    .style(Style::default().bg(row_bg))
}

// ══════════════════════════════════════════════════════════════════════════════
//  REMOTE HOSTS PANEL
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_remote(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let tick = app.tick_count;
    let phase = app.phase;

    let connected_count = app
        .remote_hosts
        .values()
        .filter(|h| h.status == ConnectionStatus::Connected)
        .count();
    let total = app.remote_hosts.len();
    let activity = if total > 0 {
        connected_count as f64 / total as f64
    } else {
        0.0
    };

    let title = format!(
        " {} Remote Hosts [{}/{}] ",
        animation::dot_spinner(tick),
        connected_count,
        total
    );
    let block = animated_block(&title, theme, phase, activity);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.remote_hosts.is_empty() {
        let wave = animation::wave_pattern(tick, inner.width as usize);
        let msg = Paragraph::new(vec![
            Line::from(Span::styled(
                " No remote hosts configured",
                Style::default().fg(theme.text_dim),
            )),
            Line::from(Span::styled(
                " Add hosts in ~/.config/pulse/config.toml [remote]",
                Style::default().fg(theme.text_dim),
            )),
            Line::from(Span::styled(wave, Style::default().fg(theme.border_dim))),
        ]);
        frame.render_widget(msg, inner);
        return;
    }

    // Sort hosts deterministically
    let mut hosts: Vec<_> = app.remote_hosts.iter().collect();
    hosts.sort_by_key(|(addr, _)| (*addr).clone());

    // We'll display each host as a mini dashboard section
    let host_count = hosts.len();
    let per_host_height = if host_count > 0 {
        (inner.height as usize / host_count).max(4)
    } else {
        4
    };

    let constraints: Vec<Constraint> = hosts
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if i < host_count - 1 {
                Constraint::Length(per_host_height as u16)
            } else {
                Constraint::Min(4)
            }
        })
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, (_addr, host)) in hosts.iter().enumerate() {
        if i >= chunks.len() {
            break;
        }
        draw_remote_host(frame, chunks[i], app, host, tick, phase);
    }
}

fn draw_remote_host(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    host: &crate::remote::RemoteHost,
    tick: u64,
    _phase: f64,
) {
    let theme = &app.theme;

    let (status_icon, status_color) = match &host.status {
        ConnectionStatus::Connected => ("●", theme.accent_success),
        ConnectionStatus::Connecting => {
            // Blink the icon for connecting state
            ("◌", theme.accent_warning)
        }
        ConnectionStatus::Error(_) => ("✖", theme.accent_error),
        ConnectionStatus::Disconnected => ("○", theme.text_dim),
    };

    let mut lines = Vec::new();

    // Host header line
    lines.push(Line::from(vec![
        Span::styled(
            format!(" {} ", status_icon),
            Style::default().fg(status_color),
        ),
        Span::styled(
            host.label(),
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ({})", host.address),
            Style::default().fg(theme.text_dim),
        ),
    ]));

    // Show data if connected
    if let Some(ref packet) = host.latest {
        let cpu_ratio = packet.cpu.global / 100.0;
        let cpu_color = theme.gradient_color(cpu_ratio);
        let mem_ratio = packet.mem.usage_ratio();
        let mem_color = theme.gradient_color(mem_ratio);

        let cpu_bar_w = 15.min(area.width.saturating_sub(30) as usize);
        let cpu_bar = animation::shimmer_bar(cpu_ratio, cpu_bar_w, tick);
        let mem_bar = animation::shimmer_bar(mem_ratio, cpu_bar_w, tick);

        lines.push(Line::from(vec![
            Span::styled("   CPU: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{:5.1}% ", packet.cpu.global),
                Style::default()
                    .fg(cpu_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("[{}]", cpu_bar),
                Style::default().fg(cpu_color),
            ),
            Span::styled("  MEM: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!(
                    "{:.1}/{:.1}G ",
                    crate::utils::bytes_to_gib(packet.mem.used),
                    crate::utils::bytes_to_gib(packet.mem.total),
                ),
                Style::default().fg(mem_color),
            ),
            Span::styled(
                format!("[{}]", mem_bar),
                Style::default().fg(mem_color),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("   Net: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("▼{}", crate::utils::format_bytes_speed(packet.net.rx_speed)),
                Style::default().fg(theme.accent_success),
            ),
            Span::styled(
                format!(" ▲{}", crate::utils::format_bytes_speed(packet.net.tx_speed)),
                Style::default().fg(theme.accent_warning),
            ),
            Span::styled(
                format!(
                    "  Load: {:.2} {:.2} {:.2}",
                    packet.cpu.load_avg.0, packet.cpu.load_avg.1, packet.cpu.load_avg.2
                ),
                Style::default().fg(theme.text_dim),
            ),
            Span::styled(
                format!("  up: {}", crate::utils::format_uptime(packet.uptime)),
                Style::default().fg(theme.text_dim),
            ),
        ]));

        // Temp + disks summary
        if let Some(temp) = packet.cpu.temperature {
            let warn = temp > 80.0;
            lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        "   Temp: {:.0}°C{}",
                        temp,
                        if warn && animation::flicker(tick, 8) { " ⚠" } else { "" }
                    ),
                    Style::default().fg(if warn {
                        theme.accent_error
                    } else {
                        theme.accent_warning
                    }),
                ),
                Span::styled(
                    format!("  IO: R {} W {}",
                        crate::utils::format_bytes_speed(packet.disk_io.read_speed),
                        crate::utils::format_bytes_speed(packet.disk_io.write_speed),
                    ),
                    Style::default().fg(theme.text_dim),
                ),
            ]));
        }
    } else {
        // Not connected — show status
        let status_msg = match &host.status {
            ConnectionStatus::Connecting => format!(" {} Connecting...", animation::dot_spinner(tick)),
            ConnectionStatus::Error(e) => format!("   Error: {}", e),
            ConnectionStatus::Disconnected => "   Disconnected".to_string(),
            _ => String::new(),
        };
        lines.push(Line::from(Span::styled(
            status_msg,
            Style::default().fg(status_color),
        )));
    }

    // Separator between hosts
    let sep_w = area.width.saturating_sub(2) as usize;
    lines.push(Line::from(Span::styled(
        format!(" {}", "─".repeat(sep_w)),
        Style::default().fg(theme.border_dim),
    )));

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

// ══════════════════════════════════════════════════════════════════════════════
//  FOOTER
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let tick = app.tick_count;

    let key_style = Style::default()
        .fg(theme.accent_tertiary)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme.text_dim);
    let sep_style = Style::default().fg(theme.border_dim);

    let mut hints = vec![
        Span::styled(" q", key_style),
        Span::styled("Quit", desc_style),
        Span::styled("│", sep_style),
        Span::styled("s", key_style),
        Span::styled("Sort", desc_style),
        Span::styled("│", sep_style),
        Span::styled("f", key_style),
        Span::styled("Filt", desc_style),
        Span::styled("│", sep_style),
        Span::styled("k", key_style),
        Span::styled("Kill", desc_style),
        Span::styled("│", sep_style),
        Span::styled("m", key_style),
        Span::styled("Mode", desc_style),
        Span::styled("│", sep_style),
        Span::styled("t", key_style),
        Span::styled("Theme", desc_style),
        Span::styled("│", sep_style),
        Span::styled("n", key_style),
        Span::styled("Net", desc_style),
        Span::styled("│", sep_style),
        Span::styled("d", key_style),
        Span::styled("Disk", desc_style),
        Span::styled("│", sep_style),
        Span::styled("g", key_style),
        Span::styled("GPU", desc_style),
        Span::styled("│", sep_style),
        Span::styled("h", key_style),
        Span::styled("Hist", desc_style),
        Span::styled("│", sep_style),
        Span::styled("c", key_style),
        Span::styled("CRT", desc_style),
        Span::styled("│", sep_style),
        Span::styled("R", key_style),
        Span::styled("Rmt", desc_style),
        Span::styled("│", sep_style),
        Span::styled("x", key_style),
        Span::styled("Heat", desc_style),
        Span::styled("│", sep_style),
        Span::styled("a", key_style),
        Span::styled("Alerts", desc_style),
        Span::styled("│", sep_style),
        Span::styled("i", key_style),
        Span::styled("Info", desc_style),
        Span::styled("│", sep_style),
        Span::styled("b", key_style),
        Span::styled("Mtrx", desc_style),
        Span::styled("│", sep_style),
        Span::styled("p", key_style),
        Span::styled("Tree", desc_style),
        Span::styled("│", sep_style),
        Span::styled("w", key_style),
        Span::styled("Cont", desc_style),
    ];

    if app.config.security.enabled {
        hints.push(Span::styled(
            " [SEC]",
            Style::default()
                .fg(if animation::flicker(tick, 10) {
                    theme.accent_error
                } else {
                    theme.accent_warning
                })
                .add_modifier(Modifier::BOLD),
        ));
    }

    if app.crt_enabled {
        hints.push(Span::styled(
            " [CRT]",
            Style::default()
                .fg(theme.accent_tertiary)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if app.config.display.matrix_bg {
        hints.push(Span::styled(
            " [MTX]",
            Style::default()
                .fg(theme.accent_success)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if app.tree_view {
        hints.push(Span::styled(
            " [TREE]",
            Style::default()
                .fg(theme.accent_secondary)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if !app.remote_hosts.is_empty() {
        let connected = app
            .remote_hosts
            .values()
            .filter(|h| h.status == crate::remote::ConnectionStatus::Connected)
            .count();
        hints.push(Span::styled(
            format!(" [R:{}/{}]", connected, app.remote_hosts.len()),
            Style::default()
                .fg(if connected > 0 {
                    theme.accent_success
                } else {
                    theme.text_dim
                })
                .add_modifier(Modifier::BOLD),
        ));
    }

    hints.push(Span::styled(
        format!("  {}:{}", animation::dot_spinner(tick), theme.id.label()),
        Style::default().fg(theme.accent_primary),
    ));

    let footer = Paragraph::new(Line::from(hints))
        .style(Style::default().bg(theme.bg_panel));
    frame.render_widget(footer, area);
}

// ══════════════════════════════════════════════════════════════════════════════
//  STARTUP SPLASH SCREEN
// ══════════════════════════════════════════════════════════════════════════════

const LOGO: &[&str] = &[
    "██████╗ ██╗   ██╗██╗     ███████╗███████╗",
    "██╔══██╗██║   ██║██║     ██╔════╝██╔════╝",
    "██████╔╝██║   ██║██║     ███████╗█████╗  ",
    "██╔═══╝ ██║   ██║██║     ╚════██║██╔══╝  ",
    "██║     ╚██████╔╝███████╗███████║███████╗",
    "╚═╝      ╚═════╝ ╚══════╝╚══════╝╚══════╝",
];

pub fn draw_splash(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let tick = app.tick_count;
    let phase = app.phase;

    // Dark full-screen background
    let bg = Block::default().style(Style::default().bg(theme.bg_dark));
    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(bg, area);

    // Total splash content height: logo (6) + gap (1) + subtitle (1) + gap (1)
    //   + tagline (1) + gap (1) + progress (1) + gap (1) + prompt (1) = 14 lines
    let content_h: u16 = 14;
    let logo_w: u16 = LOGO[0].chars().count() as u16;

    // Center the content block
    let start_y = area.y + area.height.saturating_sub(content_h) / 2;
    let start_x = area.x + area.width.saturating_sub(logo_w) / 2;

    // ── Animated logo rows ────────────────────────────────────────────────
    for (row_idx, &logo_line) in LOGO.iter().enumerate() {
        let y = start_y + row_idx as u16;
        if y >= area.y + area.height {
            break;
        }

        // Each row gets a phase offset → wave of colour sweeps downward
        let row_phase = (phase + row_idx as f64 * 0.12) % 1.0;
        let (r, g, b) = animation::rainbow_rgb(row_phase, 0.0);

        // Subtle secondary glow: slightly dimmer colour from the adjacent row
        let glow_phase = (row_phase + 0.05) % 1.0;
        let (gr, gg, gb) = animation::rainbow_rgb(glow_phase, 0.0);
        let glow_color = Color::Rgb(gr / 3, gg / 3, gb / 3);

        // Build per-character spans so the glow blends in on box-drawing chars
        let spans: Vec<Span> = logo_line
            .chars()
            .map(|ch| {
                let color = if ch == ' ' || ch == '╝' || ch == '╗' || ch == '╔' || ch == '═' || ch == '╚' {
                    glow_color
                } else {
                    Color::Rgb(r, g, b)
                };
                Span::styled(ch.to_string(), Style::default().fg(color).add_modifier(Modifier::BOLD))
            })
            .collect();

        let row_area = Rect::new(start_x, y, area.width.saturating_sub(start_x - area.x), 1);
        frame.render_widget(Paragraph::new(Line::from(spans)), row_area);
    }

    let after_logo_y = start_y + LOGO.len() as u16;

    // ── Version / tagline ─────────────────────────────────────────────────
    let (vr, vg, vb) = animation::neon_cycle(phase);
    let ver_line = format!("v{}  ·  SYSTEM MONITOR", env!("CARGO_PKG_VERSION"));
    let ver_x = area.x + area.width.saturating_sub(ver_line.len() as u16) / 2;
    let ver_area = Rect::new(ver_x, after_logo_y + 1, area.width, 1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            ver_line,
            Style::default()
                .fg(Color::Rgb(vr, vg, vb))
                .add_modifier(Modifier::BOLD),
        )),
        ver_area,
    );

    // ── Animated scan-line decoration ─────────────────────────────────────
    let scan_y = after_logo_y + 2;
    let scan_str = animation::scan_line(tick, logo_w as usize);
    let scan_x = area.x + area.width.saturating_sub(logo_w) / 2;
    let scan_area = Rect::new(scan_x, scan_y, logo_w, 1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            scan_str,
            Style::default()
                .fg(Color::Rgb(vr / 3, vg / 3, vb / 3))
                .add_modifier(Modifier::DIM),
        )),
        scan_area,
    );

    // ── Progress bar (countdown) ──────────────────────────────────────────
    let remaining = app.splash_remaining;
    // splash starts at 180, progress = 1.0 - remaining/180
    let ratio = 1.0_f64 - (remaining as f64 / 180.0_f64).clamp(0.0, 1.0);
    let bar_w = logo_w as usize;
    let filled = (ratio * bar_w as f64) as usize;
    let bar: String = (0..bar_w)
        .map(|i| {
            if i < filled {
                '█'
            } else if i == filled {
                '▌'
            } else {
                '░'
            }
        })
        .collect();
    let bar_phase = (phase + ratio * 0.5) % 1.0;
    let (br, bg_c, bb) = animation::rainbow_rgb(bar_phase, 0.0);
    let bar_x = area.x + area.width.saturating_sub(logo_w) / 2;
    let bar_area = Rect::new(bar_x, scan_y + 2, logo_w, 1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            bar,
            Style::default().fg(Color::Rgb(br, bg_c, bb)),
        )),
        bar_area,
    );

    // ── "Press any key" prompt ────────────────────────────────────────────
    let cursor = if animation::flicker(tick, 8) { "▌" } else { " " };
    let prompt = format!("Press any key to start {}", cursor);
    let prompt_x = area.x + area.width.saturating_sub(prompt.len() as u16) / 2;
    let prompt_area = Rect::new(prompt_x, scan_y + 4, area.width, 1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            prompt,
            Style::default()
                .fg(theme.text_dim)
                .add_modifier(Modifier::ITALIC),
        )),
        prompt_area,
    );
}

// ══════════════════════════════════════════════════════════════════════════════
//  FILTER INPUT OVERLAY
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_filter_input(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let tick = app.tick_count;

    // Draw a centered input box with glowing border
    let width = 50.min(area.width.saturating_sub(4));
    let height = 3;
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;
    let input_area = Rect::new(x, y, width, height);

    let glow_color = theme.glow_border_style(app.phase);

    let block = Block::default()
        .title(Span::styled(
            format!(" {} Filter (regex) ", animation::dot_spinner(tick)),
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(glow_color)
        .border_set(symbols::border::DOUBLE)
        .style(Style::default().bg(theme.bg_panel));

    // Blinking cursor
    let cursor = if animation::flicker(tick, 8) { "▌" } else { " " };

    let input = Paragraph::new(Line::from(Span::styled(
        format!("{}{}", app.filter_text, cursor),
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
        let tick = app.tick_count;
        let width = (msg.len() as u16 + 8).min(area.width.saturating_sub(4));
        let x = (area.width.saturating_sub(width)) / 2 + area.x;
        let y = area.height.saturating_sub(4) + area.y;
        let msg_area = Rect::new(x, y, width, 3);

        let (nr, ng, nb) = animation::neon_cycle(app.phase);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(nr, ng, nb)))
            .border_set(symbols::border::ROUNDED)
            .style(Style::default().bg(theme.bg_panel));

        let para = Paragraph::new(Line::from(Span::styled(
            format!(" {} {} ", animation::dot_spinner(tick), msg),
            Style::default()
                .fg(theme.accent_warning)
                .add_modifier(Modifier::BOLD),
        )))
        .block(block);

        frame.render_widget(para, msg_area);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
//  HELPERS
// ══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn styled_block<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    Block::default()
        .title(Span::styled(title, theme.title_style()))
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .style(Style::default().bg(theme.bg_dark))
}

/// Animated block with glowing border whose intensity tracks an activity metric.
fn animated_block<'a>(title: &'a str, theme: &'a Theme, phase: f64, activity: f64) -> Block<'a> {
    let (r, g, b) = animation::border_glow_color(activity, phase);
    let glow_color = Color::Rgb(r, g, b);

    // Lerp between normal border and glow based on activity
    let border_color = crate::ui::theme::lerp_color(theme.border_dim, glow_color, activity);

    Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(crate::ui::theme::lerp_color(
                    theme.accent_primary,
                    glow_color,
                    animation::breathing(phase) * 0.3,
                ))
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .border_set(symbols::border::ROUNDED)
        .style(Style::default().bg(theme.bg_dark))
}

// ══════════════════════════════════════════════════════════════════════════════
//  CPU CORE HEATMAP
// ══════════════════════════════════════════════════════════════════════════════

/// Renders a 2-D heat map: each row is a CPU core, each column a past sample.
pub fn draw_heatmap(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let phase = app.phase;
    let num_cores = app.history.cpu_per_core.len();
    let activity = app.cpu.global / 100.0;
    let block = animated_block(" ◈ CPU Core Heatmap ", theme, phase, activity);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if num_cores == 0 || inner.height < 3 || inner.width < 8 {
        return;
    }

    // Reserve 2 rows at the bottom for the legend
    let legend_height: u16 = 2;
    let grid_height = inner.height.saturating_sub(legend_height);

    // First 4 columns are the core label ("NN▕")
    let label_w: u16 = 4;
    let grid_w = inner.width.saturating_sub(label_w) as usize;

    let cores_to_show = num_cores.min(grid_height as usize);

    for (core_idx, core_history) in app.history.cpu_per_core.iter().enumerate().take(cores_to_show) {
        let data = core_history.last_n(grid_w);
        let y = inner.y + core_idx as u16;

        let label = format!("{:>2}▕", core_idx);
        let mut spans: Vec<Span> = vec![
            Span::styled(label, Style::default().fg(theme.text_dim)),
        ];

        // Pad left with cold colour when history is shorter than the grid
        let pad = grid_w.saturating_sub(data.len());
        for _ in 0..pad {
            spans.push(Span::styled("░", Style::default().fg(Color::Rgb(15, 15, 35))));
        }

        for value in &data {
            let ratio = (*value / 100.0).clamp(0.0, 1.0);
            let color = heat_color(ratio);
            let ch = if ratio < 0.05 {
                "░"
            } else if ratio < 0.35 {
                "▒"
            } else if ratio < 0.65 {
                "▓"
            } else {
                "█"
            };
            spans.push(Span::styled(ch, Style::default().fg(color)));
        }

        let row_area = Rect::new(inner.x, y, inner.width, 1);
        frame.render_widget(Paragraph::new(Line::from(spans)), row_area);
    }

    // Legend gradient strip
    let legend_y = inner.y + cores_to_show as u16 + 1;
    if legend_y < inner.y + inner.height {
        let strip_w = grid_w.min(40);
        let mut lspans: Vec<Span> = vec![
            Span::styled("    0% ", Style::default().fg(theme.text_dim)),
        ];
        for i in 0..strip_w {
            let ratio = i as f64 / strip_w as f64;
            lspans.push(Span::styled("▄", Style::default().fg(heat_color(ratio))));
        }
        lspans.push(Span::styled(" 100%", Style::default().fg(theme.text_dim)));
        let legend_area = Rect::new(inner.x, legend_y, inner.width, 1);
        frame.render_widget(Paragraph::new(Line::from(lspans)), legend_area);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
//  SYSTEM ALERTS LOG
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_alerts(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let phase = app.phase;
    let tick = app.tick_count;

    let block = animated_block(" ⚠ System Alerts ", theme, phase, 0.0);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.alerts.is_empty() {
        let wave = animation::wave_pattern(tick, inner.width as usize);
        let msg = Paragraph::new(vec![
            Line::from(Span::styled(
                " No alerts yet. Enable security mode with [!] to start tracking.",
                Style::default().fg(theme.text_dim),
            )),
            Line::from(Span::styled(wave, Style::default().fg(theme.border_dim))),
        ]);
        frame.render_widget(msg, inner);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Age  ").style(
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("").style(Style::default()),
        Cell::from("Event").style(
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let visible = inner.height.saturating_sub(1) as usize;
    let rows: Vec<Row> = app
        .alerts
        .iter()
        .rev()
        .take(visible)
        .map(|ev| {
            let age_ticks = tick.saturating_sub(ev.tick);
            // Approx: 60 FPS, data refresh ~every 30 ticks
            let secs = age_ticks / 60;
            let age_str = if secs < 60 {
                format!("{:>3}s ", secs)
            } else {
                format!("{:>2}m{:02}s", secs / 60, secs % 60)
            };
            let (icon, color) = match ev.severity {
                AlertSeverity::Info => ("ℹ", theme.accent_secondary),
                AlertSeverity::Warning => ("⚠", theme.accent_warning),
                AlertSeverity::Critical => ("✖", theme.accent_error),
            };
            Row::new(vec![
                Cell::from(age_str).style(Style::default().fg(theme.text_dim)),
                Cell::from(icon).style(Style::default().fg(color)),
                Cell::from(ev.message.clone()).style(Style::default().fg(color)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [Constraint::Length(6), Constraint::Length(2), Constraint::Min(20)],
    )
    .header(header);
    frame.render_widget(table, inner);
}

// ══════════════════════════════════════════════════════════════════════════════
//  PROCESS DETAIL POPUP
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_process_detail(frame: &mut Frame, area: Rect, app: &App) {
    let procs = app.filtered_processes();
    let proc = match procs.get(app.process_scroll) {
        Some(p) => *p,
        None => return,
    };

    let popup_w = (area.width * 70 / 100).max(60).min(area.width.saturating_sub(4));
    let popup_h = (area.height * 65 / 100).max(14).min(area.height.saturating_sub(2));
    let popup_x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_w, popup_h);

    let theme = &app.theme;
    let tick = app.tick_count;

    frame.render_widget(ratatui::widgets::Clear, popup_area);

    let cmdline = read_proc_cmdline(proc.pid);
    let fd_count = count_proc_fds(proc.pid);

    let (nr, ng, nb) = animation::neon_cycle(app.phase);
    let border_color = Color::Rgb(nr, ng, nb);

    let block = Block::default()
        .title(Span::styled(
            format!(
                " {} Process: {} (PID {})  [i/Esc] ",
                animation::dot_spinner(tick),
                proc.name,
                proc.pid
            ),
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .border_set(symbols::border::ROUNDED)
        .style(Style::default().bg(theme.bg_panel));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let label_sty = Style::default().fg(theme.text_dim);
    let val_sty = Style::default()
        .fg(theme.text_bright)
        .add_modifier(Modifier::BOLD);
    let cpu_color = theme.gradient_color(proc.cpu as f64 / 100.0);
    let mem_color = theme.gradient_color(proc.mem_mb as f64 / 8192.0);
    let sep = Span::styled(
        "─".repeat(inner.width as usize),
        Style::default().fg(theme.border_dim),
    );

    let max_cmd_w = (inner.width as usize).saturating_sub(12);
    let cmd_display = if cmdline.len() > max_cmd_w {
        format!("{}…", &cmdline[..max_cmd_w.saturating_sub(1)])
    } else {
        cmdline
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Name:      ", label_sty),
            Span::styled(
                proc.name.clone(),
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" PID:       ", label_sty),
            Span::styled(proc.pid.to_string(), val_sty),
            Span::styled("   User: ", label_sty),
            Span::styled(proc.user.clone(), val_sty),
        ]),
        Line::from(sep.clone()),
        Line::from(vec![
            Span::styled(" CPU:       ", label_sty),
            Span::styled(
                format!("{:.1}%", proc.cpu),
                Style::default().fg(cpu_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Memory: ", label_sty),
            Span::styled(
                format!("{:.1} MB", proc.mem_mb),
                Style::default().fg(mem_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Threads:   ", label_sty),
            Span::styled(
                proc.threads
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "?".into()),
                val_sty,
            ),
            Span::styled("   Nice:    ", label_sty),
            Span::styled(proc.nice.to_string(), val_sty),
        ]),
        Line::from(vec![
            Span::styled(" Status:    ", label_sty),
            Span::styled(proc.status.clone(), val_sty),
            Span::styled("   Open FDs: ", label_sty),
            Span::styled(
                fd_count
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "n/a".into()),
                val_sty,
            ),
        ]),
        Line::from(sep.clone()),
        Line::from(vec![
            Span::styled(" Cmdline:   ", label_sty),
            Span::styled(cmd_display, Style::default().fg(theme.accent_tertiary)),
        ]),
    ];

    // Inline braille sparkline for this process's CPU history
    if let Some(ring) = app.history.process_cpu.get(&proc.pid) {
        let hist_w = (inner.width as usize).saturating_sub(14);
        let hist_data = ring.last_n(hist_w);
        let spark = animation::braille_sparkline(&hist_data, 100.0, hist_data.len());
        lines.push(Line::from(sep));
        lines.push(Line::from(vec![
            Span::styled(" CPU hist:  ", label_sty),
            Span::styled(spark, Style::default().fg(cpu_color)),
        ]));
    }

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().bg(theme.bg_panel));
    frame.render_widget(para, inner);
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Heat-map colour ramp: deep navy → teal → yellow → orange → bright red.
fn heat_color(ratio: f64) -> Color {
    if ratio < 0.25 {
        crate::ui::theme::lerp_color(
            Color::Rgb(0, 20, 80),
            Color::Rgb(0, 160, 120),
            ratio / 0.25,
        )
    } else if ratio < 0.50 {
        crate::ui::theme::lerp_color(
            Color::Rgb(0, 160, 120),
            Color::Rgb(200, 200, 0),
            (ratio - 0.25) / 0.25,
        )
    } else if ratio < 0.75 {
        crate::ui::theme::lerp_color(
            Color::Rgb(200, 200, 0),
            Color::Rgb(240, 80, 0),
            (ratio - 0.50) / 0.25,
        )
    } else {
        crate::ui::theme::lerp_color(
            Color::Rgb(240, 80, 0),
            Color::Rgb(255, 20, 20),
            (ratio - 0.75) / 0.25,
        )
    }
}

/// Read `/proc/<pid>/cmdline` and convert null bytes to spaces.
fn read_proc_cmdline(pid: u32) -> String {
    std::fs::read(format!("/proc/{}/cmdline", pid))
        .map(|bytes| {
            bytes
                .iter()
                .map(|&b| if b == 0 { ' ' } else { b as char })
                .collect::<String>()
                .trim()
                .to_string()
        })
        .unwrap_or_default()
}

/// Count entries in `/proc/<pid>/fd` to approximate open file descriptors.
fn count_proc_fds(pid: u32) -> Option<usize> {
    std::fs::read_dir(format!("/proc/{}/fd", pid))
        .ok()
        .map(|iter| iter.count())
}

// ══════════════════════════════════════════════════════════════════════════════
//  CONTAINER MONITOR
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_containers(frame: &mut Frame, area: Rect, app: &App) {
    use crate::system::container::{ContainerRuntime, ContainerState};

    let theme = &app.theme;
    let tick = app.tick_count;
    let phase = app.phase;
    let snapshot = &app.containers;

    let running_count = snapshot
        .containers
        .iter()
        .filter(|c| c.state == ContainerState::Running)
        .count();

    let runtime_label = match &snapshot.runtime {
        Some(ContainerRuntime::Docker) => "Docker",
        Some(ContainerRuntime::Podman) => "Podman",
        None => "N/A",
    };

    let activity = if snapshot.containers.is_empty() {
        0.0
    } else {
        running_count as f64 / snapshot.containers.len() as f64
    };

    let title = format!(
        " {} Containers via {} [{}/{}] ",
        animation::dot_spinner(tick),
        runtime_label,
        running_count,
        snapshot.containers.len(),
    );
    let block = animated_block(&title, theme, phase, activity);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if !snapshot.available {
        let msg = Paragraph::new(vec![
            Line::from(Span::styled(
                " No container runtime detected",
                Style::default().fg(theme.text_dim),
            )),
            Line::from(Span::styled(
                " Install Docker or Podman to enable container monitoring",
                Style::default().fg(theme.text_dim),
            )),
        ]);
        frame.render_widget(msg, inner);
        return;
    }

    if snapshot.containers.is_empty() {
        let msg = Paragraph::new(Span::styled(
            " No containers found",
            Style::default().fg(theme.text_dim),
        ));
        frame.render_widget(msg, inner);
        return;
    }

    let header_cells = ["ID", "Name", "Image", "State", "CPU%", "MEM(MB)", "Status"]
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

    let visible_height = inner.height.saturating_sub(2) as usize;

    let rows: Vec<Row> = snapshot
        .containers
        .iter()
        .skip(app.container_scroll)
        .take(visible_height)
        .enumerate()
        .map(|(idx, c)| {
            let state_color = match c.state {
                ContainerState::Running => theme.accent_success,
                ContainerState::Paused => theme.accent_warning,
                ContainerState::Exited => theme.accent_error,
                _ => theme.text_dim,
            };

            let state_str = match c.state {
                ContainerState::Running => "Running",
                ContainerState::Paused => "Paused",
                ContainerState::Exited => "Exited",
                ContainerState::Created => "Created",
                ContainerState::Restarting => "Restart",
                ContainerState::Unknown => "???",
            };

            let cpu_str = c
                .cpu_pct
                .map(|v| format!("{:.1}", v))
                .unwrap_or_else(|| "-".into());
            let mem_str = c
                .mem_usage_mb
                .map(|v| format!("{:.1}", v))
                .unwrap_or_else(|| "-".into());

            let row_bg = if idx % 2 == 0 {
                theme.bg_dark
            } else {
                theme.bg_panel
            };

            Row::new(vec![
                Cell::from(c.id.clone()).style(Style::default().fg(theme.text_dim)),
                Cell::from(utils::truncate_str(&c.name, 20))
                    .style(Style::default().fg(theme.text_bright)),
                Cell::from(utils::truncate_str(&c.image, 24))
                    .style(Style::default().fg(theme.accent_tertiary)),
                Cell::from(state_str).style(
                    Style::default()
                        .fg(state_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(cpu_str).style(Style::default().fg(theme.accent_secondary)),
                Cell::from(mem_str).style(Style::default().fg(theme.accent_secondary)),
                Cell::from(utils::truncate_str(&c.status, 20))
                    .style(Style::default().fg(theme.text_dim)),
            ])
            .style(Style::default().bg(row_bg))
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(13),
            Constraint::Length(20),
            Constraint::Min(16),
            Constraint::Length(8),
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Length(20),
        ],
    )
    .header(header)
    .row_highlight_style(
        Style::default()
            .fg(theme.bg_dark)
            .bg(theme.accent_primary)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_widget(table, inner);
}

// ══════════════════════════════════════════════════════════════════════════════
//  FAN MONITOR PANEL
// ══════════════════════════════════════════════════════════════════════════════

pub fn draw_fans(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let tick = app.tick_count;
    let phase = app.phase;

    // Calculate average fan activity for border animation
    let fan_activity = if app.fans.available && !app.fans.fans.is_empty() {
        let total_pwm: f32 = app
            .fans
            .fans
            .iter()
            .filter_map(|f| f.pwm_pct())
            .sum();
        let count = app.fans.fans.iter().filter(|f| f.pwm.is_some()).count();
        if count > 0 {
            (total_pwm / count as f32) / 100.0
        } else {
            0.5 // Default activity if no PWM data
        }
    } else {
        0.0
    };

    let block = animated_block(" 🌀 Fan Monitor ", theme, phase, fan_activity as f64);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if !app.fans.available {
        let wave = animation::wave_pattern(tick, inner.width as usize);
        let msg = Paragraph::new(vec![
            Line::from(Span::styled(
                " No fans detected (hwmon)",
                Style::default().fg(theme.text_dim),
            )),
            Line::from(Span::styled(
                " Check /sys/class/hwmon/ for fan sensors",
                Style::default().fg(theme.text_dim),
            )),
            Line::from(Span::styled(wave, Style::default().fg(theme.border_dim))),
        ]);
        frame.render_widget(msg, inner);
        return;
    }

    let mut lines = Vec::new();
    let bar_width = 20;

    // Show laptop brand and fan mode if detected
    let mut header_spans = Vec::new();
    if let Some(brand) = &app.fans.laptop_brand {
        header_spans.push(Span::styled(
            format!(" {} ", brand),
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(mode) = &app.fans.fan_mode {
        let mode_color = match mode.to_lowercase().as_str() {
            "auto" | "balanced" => theme.accent_secondary,
            "silent" => theme.accent_tertiary,
            "advanced" | "turbo" => theme.accent_warning,
            _ => theme.text_dim,
        };
        header_spans.push(Span::styled(" │ ", Style::default().fg(theme.border_dim)));
        header_spans.push(Span::styled("Mode: ", Style::default().fg(theme.text_dim)));
        header_spans.push(Span::styled(
            mode.to_uppercase(),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(boost) = app.fans.cooler_boost {
        if boost {
            header_spans.push(Span::styled(
                "  BOOST ",
                Style::default()
                    .fg(theme.accent_error)
                    .add_modifier(Modifier::BOLD),
            ));
            if animation::flicker(tick, 6) {
                header_spans.push(Span::styled("🔥", Style::default()));
            }
        }
    }
    if !header_spans.is_empty() {
        lines.push(Line::from(header_spans));
        lines.push(Line::from(""));
    }

    // Group fans by device
    let mut current_device = String::new();

    for fan in &app.fans.fans {
        // Add device header when device changes
        if fan.device_name != current_device {
            if !current_device.is_empty() {
                lines.push(Line::from("")); // Separator between devices
            }
            current_device = fan.device_name.clone();
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} {} ", animation::spinner(tick), fan.device_name),
                    Style::default()
                        .fg(theme.accent_primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        // Determine speed (percentage or RPM-based)
        let speed_pct = fan.effective_speed_pct();
        let rpm = fan.rpm_or_zero();
        let has_pct = fan.speed_pct.is_some();

        // Color based on speed
        let speed_color = if fan.read_error {
            theme.accent_warning
        } else if let Some(pct) = speed_pct {
            if pct < 1.0 {
                theme.text_dim // Off/idle
            } else if pct < 30.0 {
                theme.accent_tertiary // Low
            } else if pct < 70.0 {
                theme.accent_secondary // Normal
            } else {
                theme.accent_warning // High
            }
        } else if rpm == 0 {
            theme.text_dim
        } else {
            theme.accent_secondary
        };

        // Build speed bar
        let ratio = speed_pct.map(|p| p as f64 / 100.0).unwrap_or(0.5);
        let bar = if speed_pct.is_some() {
            animation::shimmer_bar(ratio, bar_width, tick)
        } else if let Some(pwm_pct) = fan.pwm_pct() {
            animation::gradient_bar(pwm_pct as f64 / 100.0, bar_width, tick)
        } else {
            "─".repeat(bar_width)
        };

        // Spinning indicator
        let is_running = fan.is_running();
        let spin_char = if fan.read_error {
            "⚠"
        } else if is_running {
            match (tick / 4) % 4 {
                0 => "◐",
                1 => "◓",
                2 => "◑",
                _ => "◒",
            }
        } else {
            "○"
        };

        // Speed display string - show both RPM and % when available
        let speed_str = if fan.read_error {
            "    N/A    ".to_string()
        } else if let Some(rpm) = fan.rpm {
            if let Some(pct) = speed_pct {
                format!("{:>5} RPM {:>3.0}%", rpm, pct)
            } else {
                format!("{:>5} RPM     ", rpm)
            }
        } else if has_pct {
            format!("      {:>5.0}%    ", speed_pct.unwrap_or(0.0))
        } else {
            "    OFF    ".to_string()
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", spin_char),
                Style::default().fg(if fan.read_error {
                    theme.accent_warning
                } else if is_running {
                    theme.accent_tertiary
                } else {
                    theme.text_dim
                }),
            ),
            Span::styled(
                format!("{:<12} ", fan.label),
                Style::default().fg(theme.text_bright),
            ),
            Span::styled(speed_str, Style::default().fg(speed_color)),
            Span::styled(
                format!("[{}]", bar),
                Style::default().fg(theme.gradient_color(ratio)),
            ),
        ]));

        // Show PWM and mode info if available
        let mut detail_spans = Vec::new();

        // Show read error hint
        if fan.read_error {
            detail_spans.push(Span::styled(
                "    (device busy)",
                Style::default().fg(theme.text_dim),
            ));
        }

        if let Some(pwm_pct) = fan.pwm_pct() {
            detail_spans.push(Span::styled(
                format!("    PWM: {:>5.1}%", pwm_pct),
                Style::default().fg(theme.text_dim),
            ));
        }

        if let Some(mode_label) = fan.pwm_mode_label() {
            let mode_color = match mode_label {
                "Auto" => theme.accent_secondary,
                "Manual" => theme.accent_warning,
                "Off" => theme.accent_error,
                _ => theme.text_dim,
            };
            detail_spans.push(Span::styled(
                format!("  Mode: {}", mode_label),
                Style::default().fg(mode_color),
            ));
        }

        if let (Some(min), Some(max)) = (fan.min_rpm, fan.max_rpm) {
            detail_spans.push(Span::styled(
                format!("  Range: {}-{} RPM", min, max),
                Style::default().fg(theme.text_dim),
            ));
        }

        // ThinkPad fan level
        if let Some(level) = &fan.level {
            let level_color = match level.as_str() {
                "auto" => theme.accent_secondary,
                "full-speed" | "disengaged" => theme.accent_error,
                _ => theme.text_dim,
            };
            detail_spans.push(Span::styled(
                format!("  Level: {}", level),
                Style::default().fg(level_color),
            ));
        }

        if !detail_spans.is_empty() {
            lines.push(Line::from(detail_spans));
        }
    }

    // Summary footer
    lines.push(Line::from(""));
    let total_fans = app.fans.fans.len();
    // Count running fans (either by speed_pct or RPM)
    let running_fans = app
        .fans
        .fans
        .iter()
        .filter(|f| {
            if let Some(pct) = f.speed_pct {
                pct > 0
            } else if let Some(rpm) = f.rpm {
                rpm > 0
            } else {
                false
            }
        })
        .count();
    let unreadable_fans = app.fans.fans.iter().filter(|f| f.read_error).count();

    // Calculate average speed percentage
    let speeds: Vec<f32> = app
        .fans
        .fans
        .iter()
        .filter_map(|f| f.effective_speed_pct())
        .filter(|&p| p > 0.0)
        .collect();
    let avg_speed = if !speeds.is_empty() {
        speeds.iter().sum::<f32>() / speeds.len() as f32
    } else {
        0.0
    };

    let mut summary_spans = vec![
        Span::styled(" ─── ", Style::default().fg(theme.border_dim)),
        Span::styled(
            format!("Total: {} fans", total_fans),
            Style::default().fg(theme.text_dim),
        ),
        Span::styled(" │ ", Style::default().fg(theme.border_dim)),
        Span::styled(
            format!("Running: {}", running_fans),
            Style::default().fg(if running_fans > 0 {
                theme.accent_secondary
            } else {
                theme.accent_warning
            }),
        ),
    ];

    if unreadable_fans > 0 {
        summary_spans.push(Span::styled(" │ ", Style::default().fg(theme.border_dim)));
        summary_spans.push(Span::styled(
            format!("Busy: {}", unreadable_fans),
            Style::default().fg(theme.accent_warning),
        ));
    }

    if avg_speed > 0.0 {
        summary_spans.push(Span::styled(" │ ", Style::default().fg(theme.border_dim)));
        summary_spans.push(Span::styled(
            format!("Avg: {:.0}%", avg_speed),
            Style::default().fg(theme.accent_tertiary),
        ));
    }

    lines.push(Line::from(summary_spans));

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}
