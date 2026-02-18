//! Theme engine. Supports multiple color themes with runtime switching.
//! Themes are identified by name and can be configured via TOML.

#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};

// ── Theme identifiers ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeId {
    Neon,
    Monochrome,
    Retro,
}

impl ThemeId {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "monochrome" | "mono" => Self::Monochrome,
            "retro" | "green" => Self::Retro,
            _ => Self::Neon,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Neon => Self::Monochrome,
            Self::Monochrome => Self::Retro,
            Self::Retro => Self::Neon,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Neon => "Neon",
            Self::Monochrome => "Mono",
            Self::Retro => "Retro",
        }
    }
}

// ── Theme palette ────────────────────────────────────────────────────────────

/// A complete color palette defining the look of the application.
#[derive(Debug, Clone)]
pub struct Theme {
    pub id: ThemeId,
    // Background
    pub bg_dark: Color,
    pub bg_panel: Color,
    // Borders & text
    pub border_dim: Color,
    pub text_dim: Color,
    pub text_bright: Color,
    // Accent colors
    pub accent_primary: Color,
    pub accent_secondary: Color,
    pub accent_tertiary: Color,
    pub accent_warning: Color,
    pub accent_error: Color,
    pub accent_success: Color,
    // Chart gradient stops
    pub gradient: [Color; 4],
}

impl Theme {
    pub fn from_id(id: ThemeId) -> Self {
        match id {
            ThemeId::Neon => Self::neon(),
            ThemeId::Monochrome => Self::monochrome(),
            ThemeId::Retro => Self::retro(),
        }
    }

    // ── Dark neon (default) ──────────────────────────────────────────────

    fn neon() -> Self {
        Self {
            id: ThemeId::Neon,
            bg_dark: Color::Rgb(10, 10, 18),
            bg_panel: Color::Rgb(16, 16, 28),
            border_dim: Color::Rgb(40, 40, 70),
            text_dim: Color::Rgb(100, 100, 140),
            text_bright: Color::Rgb(200, 200, 230),
            accent_primary: Color::Rgb(0, 255, 255),     // Cyan
            accent_secondary: Color::Rgb(191, 64, 255),   // Purple
            accent_tertiary: Color::Rgb(255, 16, 240),    // Pink
            accent_warning: Color::Rgb(255, 165, 0),      // Orange
            accent_error: Color::Rgb(255, 55, 55),        // Red
            accent_success: Color::Rgb(57, 255, 20),      // Green
            gradient: [
                Color::Rgb(57, 255, 20),
                Color::Rgb(0, 255, 255),
                Color::Rgb(191, 64, 255),
                Color::Rgb(255, 16, 240),
            ],
        }
    }

    // ── Minimal monochrome ───────────────────────────────────────────────

    fn monochrome() -> Self {
        Self {
            id: ThemeId::Monochrome,
            bg_dark: Color::Rgb(0, 0, 0),
            bg_panel: Color::Rgb(15, 15, 15),
            border_dim: Color::Rgb(60, 60, 60),
            text_dim: Color::Rgb(120, 120, 120),
            text_bright: Color::Rgb(220, 220, 220),
            accent_primary: Color::Rgb(200, 200, 200),
            accent_secondary: Color::Rgb(160, 160, 160),
            accent_tertiary: Color::Rgb(180, 180, 180),
            accent_warning: Color::Rgb(200, 200, 140),
            accent_error: Color::Rgb(220, 100, 100),
            accent_success: Color::Rgb(180, 220, 180),
            gradient: [
                Color::Rgb(80, 80, 80),
                Color::Rgb(130, 130, 130),
                Color::Rgb(180, 180, 180),
                Color::Rgb(240, 240, 240),
            ],
        }
    }

    // ── Retro green terminal ─────────────────────────────────────────────

    fn retro() -> Self {
        Self {
            id: ThemeId::Retro,
            bg_dark: Color::Rgb(0, 10, 0),
            bg_panel: Color::Rgb(0, 15, 0),
            border_dim: Color::Rgb(0, 50, 0),
            text_dim: Color::Rgb(0, 100, 0),
            text_bright: Color::Rgb(0, 220, 0),
            accent_primary: Color::Rgb(0, 255, 0),
            accent_secondary: Color::Rgb(0, 200, 0),
            accent_tertiary: Color::Rgb(80, 255, 80),
            accent_warning: Color::Rgb(180, 255, 0),
            accent_error: Color::Rgb(255, 80, 0),
            accent_success: Color::Rgb(0, 255, 80),
            gradient: [
                Color::Rgb(0, 80, 0),
                Color::Rgb(0, 140, 0),
                Color::Rgb(0, 200, 0),
                Color::Rgb(0, 255, 0),
            ],
        }
    }

    // ── Style helpers ────────────────────────────────────────────────────

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border_dim)
    }

    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.accent_primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn highlight_style(&self) -> Style {
        Style::default()
            .fg(self.bg_dark)
            .bg(self.accent_primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn dim_style(&self) -> Style {
        Style::default().fg(self.text_dim)
    }

    pub fn bright_style(&self) -> Style {
        Style::default().fg(self.text_bright)
    }

    pub fn error_style(&self) -> Style {
        Style::default()
            .fg(self.accent_error)
            .add_modifier(Modifier::BOLD)
    }

    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.accent_warning)
    }

    pub fn success_style(&self) -> Style {
        Style::default().fg(self.accent_success)
    }

    /// Pick a gradient color based on a 0.0–1.0 ratio.
    pub fn gradient_color(&self, ratio: f64) -> Color {
        let stops = &self.gradient;
        let t = ratio.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
        let idx = (t as usize).min(stops.len() - 2);
        let frac = t - idx as f64;
        lerp_color(stops[idx], stops[idx + 1], frac)
    }
}

/// Linear interpolation between two RGB colors.
pub fn lerp_color(a: Color, b: Color, t: f64) -> Color {
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
    (a as f64 + (b as f64 - a as f64) * t).round().clamp(0.0, 255.0) as u8
}
