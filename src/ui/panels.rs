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

use crate::app::App;
use crate::remote::ConnectionStatus;
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
    let title = format!(
        " {} Processes [{}] sort:{}{} ",
        animation::dot_spinner(tick),
        proc_count,
        sort_label,
        filter_info
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

    let visible_procs = app.filtered_processes();
    let visible_height = area.height.saturating_sub(4) as usize;

    let rows: Vec<Row> = visible_procs
        .iter()
        .skip(app.process_scroll)
        .take(visible_height)
        .enumerate()
        .map(|(idx, p)| {
            let cpu_color = if p.anomaly.high_cpu {
                // Flicker effect for anomalous processes
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

            // Subtle alternating row background
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
            .style(Style::default().bg(row_bg))
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
    .row_highlight_style(
        Style::default()
            .fg(theme.bg_dark)
            .bg(theme.accent_primary)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_widget(table, area);
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
