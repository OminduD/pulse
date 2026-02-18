//! Neon cyberpunk color palette used across all widgets.

use ratatui::style::{Color, Modifier, Style};

// ── Primary palette ──────────────────────────────────────────────────────────

pub const NEON_GREEN: Color = Color::Rgb(57, 255, 20);
pub const NEON_CYAN: Color = Color::Rgb(0, 255, 255);
pub const NEON_PURPLE: Color = Color::Rgb(191, 64, 255);
pub const NEON_PINK: Color = Color::Rgb(255, 16, 240);
pub const NEON_ORANGE: Color = Color::Rgb(255, 165, 0);
pub const NEON_YELLOW: Color = Color::Rgb(255, 255, 0);
pub const NEON_RED: Color = Color::Rgb(255, 55, 55);

pub const BG_DARK: Color = Color::Rgb(10, 10, 18);
pub const BG_PANEL: Color = Color::Rgb(16, 16, 28);
pub const BORDER_DIM: Color = Color::Rgb(40, 40, 70);
pub const TEXT_DIM: Color = Color::Rgb(100, 100, 140);
pub const TEXT_BRIGHT: Color = Color::Rgb(200, 200, 230);

// ── Gradient stops for CPU chart (green → cyan → purple → pink) ─────────────

pub const CPU_GRADIENT: [Color; 4] = [NEON_GREEN, NEON_CYAN, NEON_PURPLE, NEON_PINK];

// ── Helper constructors ──────────────────────────────────────────────────────

pub fn border_style() -> Style {
    Style::default().fg(BORDER_DIM)
}

pub fn title_style() -> Style {
    Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD)
}

pub fn highlight_style() -> Style {
    Style::default()
        .fg(BG_DARK)
        .bg(NEON_CYAN)
        .add_modifier(Modifier::BOLD)
}

/// Pick a gradient color based on a 0.0–1.0 ratio.
pub fn gradient_color(ratio: f64) -> Color {
    let stops = &CPU_GRADIENT;
    let t = ratio.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
    let idx = (t as usize).min(stops.len() - 2);
    let frac = t - idx as f64;
    lerp_color(stops[idx], stops[idx + 1], frac)
}

/// Linear interpolation between two RGB colors.
fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    if let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (a, b) {
        Color::Rgb(
            lerp_u8(r1, r2, t),
            lerp_u8(g1, g2, t),
            lerp_u8(b1, b2, t),
        )
    } else {
        a
    }
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t).round() as u8
}
