//! Shared utility functions used across the application.

#![allow(dead_code)]

/// Format bytes/sec into a human-readable speed string.
pub fn format_bytes_speed(bytes_per_sec: f64) -> String {
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

/// Format a byte count as a total (GB/MB/KB/B).
pub fn format_bytes_total(bytes: u64) -> String {
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

/// Format bytes as GiB.
pub fn bytes_to_gib(bytes: u64) -> f64 {
    bytes as f64 / 1_073_741_824.0
}

/// Truncate a string to `max_len`, appending '…' if truncated.
pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

/// Build a small ASCII progress bar: `████░░░░░░`
pub fn mini_bar(ratio: f64, width: usize) -> String {
    let filled = (ratio.clamp(0.0, 1.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Return a single Unicode block character for a 0.0–1.0 ratio.
pub fn bar_glyph(ratio: f64) -> &'static str {
    match (ratio.clamp(0.0, 1.0) * 8.0) as u8 {
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

/// Format uptime in seconds to "Xd Xh Xm Xs".
pub fn format_uptime(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if d > 0 {
        format!("{}d {}h {}m {}s", d, h, m, s)
    } else if h > 0 {
        format!("{}h {}m {}s", h, m, s)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}

/// Circular meter as ASCII art lines. Returns a Vec of strings.
/// `ratio` is 0.0–1.0, `label` is the center text.
pub fn circular_meter(ratio: f64, label: &str) -> Vec<String> {
    let r = ratio.clamp(0.0, 1.0);
    let filled = (r * 12.0).round() as usize;
    let _chars = "◜◝◞◟";
    let _segments: Vec<char> = "╭─╮│╯─╰│╭───╮".chars().collect();
    // Simple ASCII circle representation
    let top = if filled >= 3 { "╭───╮" } else { "╭   ╮" };
    let mid = format!("│{:^3}│", label);
    let bot = if filled >= 6 { "╰───╯" } else { "╰   ╯" };
    vec![top.to_string(), mid, bot.to_string()]
}
