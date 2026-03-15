//! Theme engine. Supports multiple color themes with runtime switching.
//! Themes are identified by name and can be configured via TOML.

#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};

// ── Theme identifiers ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeId {
    TokyoNight,
    CatppuccinMocha,
    GruvboxDark,
    RosePine,
    Nord,
    Kanagawa,
    Dracula,
    Everforest,
    OneDark,
    Moonfly,
}

impl ThemeId {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "tokyonight" | "tokyo" | "tokyo_night" => Self::TokyoNight,
            "catppuccin" | "mocha" | "catppuccin_mocha" => Self::CatppuccinMocha,
            "gruvbox" | "gruvbox_dark" => Self::GruvboxDark,
            "rosepine" | "rose_pine" | "rose" => Self::RosePine,
            "nord" => Self::Nord,
            "kanagawa" => Self::Kanagawa,
            "dracula" => Self::Dracula,
            "everforest" => Self::Everforest,
            "onedark" | "one_dark" => Self::OneDark,
            "moonfly" => Self::Moonfly,
            _ => Self::TokyoNight,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::TokyoNight => Self::CatppuccinMocha,
            Self::CatppuccinMocha => Self::GruvboxDark,
            Self::GruvboxDark => Self::RosePine,
            Self::RosePine => Self::Nord,
            Self::Nord => Self::Kanagawa,
            Self::Kanagawa => Self::Dracula,
            Self::Dracula => Self::Everforest,
            Self::Everforest => Self::OneDark,
            Self::OneDark => Self::Moonfly,
            Self::Moonfly => Self::TokyoNight,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::TokyoNight => "Tokyo Night",
            Self::CatppuccinMocha => "Catppuccin",
            Self::GruvboxDark => "Gruvbox",
            Self::RosePine => "Rose Pine",
            Self::Nord => "Nord",
            Self::Kanagawa => "Kanagawa",
            Self::Dracula => "Dracula",
            Self::Everforest => "Everforest",
            Self::OneDark => "One Dark",
            Self::Moonfly => "Moonfly",
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
    pub border_glow: Color,
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
    // Extra effects
    pub sparkline_fill: Color,
    pub header_accent: Color,
}

impl Theme {
    pub fn from_id(id: ThemeId) -> Self {
        match id {
            ThemeId::TokyoNight => Self::tokyo_night(),
            ThemeId::CatppuccinMocha => Self::catppuccin_mocha(),
            ThemeId::GruvboxDark => Self::gruvbox_dark(),
            ThemeId::RosePine => Self::rose_pine(),
            ThemeId::Nord => Self::nord(),
            ThemeId::Kanagawa => Self::kanagawa(),
            ThemeId::Dracula => Self::dracula(),
            ThemeId::Everforest => Self::everforest(),
            ThemeId::OneDark => Self::one_dark(),
            ThemeId::Moonfly => Self::moonfly(),
        }
    }

    // ── Tokyo Night ──────────────────────────────────────────────────────

    fn tokyo_night() -> Self {
        Self {
            id: ThemeId::TokyoNight,
            bg_dark: Color::Rgb(26, 27, 38),
            bg_panel: Color::Rgb(31, 35, 53),
            border_dim: Color::Rgb(86, 95, 137),
            border_glow: Color::Rgb(122, 162, 247),
            text_dim: Color::Rgb(86, 95, 137),
            text_bright: Color::Rgb(192, 202, 245),
            accent_primary: Color::Rgb(122, 162, 247),    // blue
            accent_secondary: Color::Rgb(187, 154, 247),  // purple
            accent_tertiary: Color::Rgb(125, 207, 255),   // cyan
            accent_warning: Color::Rgb(224, 175, 104),    // yellow
            accent_error: Color::Rgb(247, 118, 142),      // red
            accent_success: Color::Rgb(158, 206, 106),    // green
            gradient: [
                Color::Rgb(158, 206, 106),
                Color::Rgb(125, 207, 255),
                Color::Rgb(122, 162, 247),
                Color::Rgb(187, 154, 247),
            ],
            sparkline_fill: Color::Rgb(52, 59, 88),
            header_accent: Color::Rgb(122, 162, 247),
        }
    }

    // ── Catppuccin Mocha ─────────────────────────────────────────────────

    fn catppuccin_mocha() -> Self {
        Self {
            id: ThemeId::CatppuccinMocha,
            bg_dark: Color::Rgb(24, 24, 37),
            bg_panel: Color::Rgb(30, 30, 46),
            border_dim: Color::Rgb(88, 91, 112),
            border_glow: Color::Rgb(137, 180, 250),
            text_dim: Color::Rgb(88, 91, 112),
            text_bright: Color::Rgb(205, 214, 244),
            accent_primary: Color::Rgb(137, 180, 250),    // blue
            accent_secondary: Color::Rgb(203, 166, 247),  // mauve
            accent_tertiary: Color::Rgb(148, 226, 213),   // teal
            accent_warning: Color::Rgb(249, 226, 175),    // yellow
            accent_error: Color::Rgb(243, 139, 168),      // red
            accent_success: Color::Rgb(166, 227, 161),    // green
            gradient: [
                Color::Rgb(166, 227, 161),
                Color::Rgb(148, 226, 213),
                Color::Rgb(137, 180, 250),
                Color::Rgb(203, 166, 247),
            ],
            sparkline_fill: Color::Rgb(49, 50, 68),
            header_accent: Color::Rgb(137, 180, 250),
        }
    }

    // ── Gruvbox Dark ─────────────────────────────────────────────────────

    fn gruvbox_dark() -> Self {
        Self {
            id: ThemeId::GruvboxDark,
            bg_dark: Color::Rgb(29, 32, 33),
            bg_panel: Color::Rgb(40, 40, 40),
            border_dim: Color::Rgb(80, 73, 69),
            border_glow: Color::Rgb(215, 153, 33),
            text_dim: Color::Rgb(102, 92, 84),
            text_bright: Color::Rgb(235, 219, 178),
            accent_primary: Color::Rgb(215, 153, 33),     // yellow
            accent_secondary: Color::Rgb(204, 36, 29),    // red
            accent_tertiary: Color::Rgb(69, 133, 136),    // aqua
            accent_warning: Color::Rgb(214, 93, 14),      // orange
            accent_error: Color::Rgb(251, 73, 52),        // bright red
            accent_success: Color::Rgb(152, 151, 26),     // green
            gradient: [
                Color::Rgb(152, 151, 26),
                Color::Rgb(69, 133, 136),
                Color::Rgb(215, 153, 33),
                Color::Rgb(214, 93, 14),
            ],
            sparkline_fill: Color::Rgb(60, 56, 54),
            header_accent: Color::Rgb(215, 153, 33),
        }
    }

    // ── Rose Pine ────────────────────────────────────────────────────────

    fn rose_pine() -> Self {
        Self {
            id: ThemeId::RosePine,
            bg_dark: Color::Rgb(25, 23, 36),
            bg_panel: Color::Rgb(30, 28, 48),
            border_dim: Color::Rgb(86, 82, 110),
            border_glow: Color::Rgb(235, 188, 186),
            text_dim: Color::Rgb(110, 106, 134),
            text_bright: Color::Rgb(224, 222, 244),
            accent_primary: Color::Rgb(235, 188, 186),    // rose
            accent_secondary: Color::Rgb(196, 167, 231),  // iris
            accent_tertiary: Color::Rgb(156, 207, 216),   // foam
            accent_warning: Color::Rgb(246, 193, 119),    // gold
            accent_error: Color::Rgb(235, 111, 146),      // love
            accent_success: Color::Rgb(49, 116, 143),     // pine
            gradient: [
                Color::Rgb(49, 116, 143),
                Color::Rgb(156, 207, 216),
                Color::Rgb(235, 188, 186),
                Color::Rgb(196, 167, 231),
            ],
            sparkline_fill: Color::Rgb(38, 35, 58),
            header_accent: Color::Rgb(235, 188, 186),
        }
    }

    // ── Nord ─────────────────────────────────────────────────────────────

    fn nord() -> Self {
        Self {
            id: ThemeId::Nord,
            bg_dark: Color::Rgb(46, 52, 64),
            bg_panel: Color::Rgb(59, 66, 82),
            border_dim: Color::Rgb(67, 76, 94),
            border_glow: Color::Rgb(136, 192, 208),
            text_dim: Color::Rgb(76, 86, 106),
            text_bright: Color::Rgb(236, 239, 244),
            accent_primary: Color::Rgb(136, 192, 208),    // frost cyan
            accent_secondary: Color::Rgb(129, 161, 193),  // frost blue
            accent_tertiary: Color::Rgb(143, 188, 187),   // frost teal
            accent_warning: Color::Rgb(235, 203, 139),    // aurora yellow
            accent_error: Color::Rgb(191, 97, 106),       // aurora red
            accent_success: Color::Rgb(163, 190, 140),    // aurora green
            gradient: [
                Color::Rgb(163, 190, 140),
                Color::Rgb(143, 188, 187),
                Color::Rgb(136, 192, 208),
                Color::Rgb(129, 161, 193),
            ],
            sparkline_fill: Color::Rgb(67, 76, 94),
            header_accent: Color::Rgb(136, 192, 208),
        }
    }

    // ── Kanagawa ─────────────────────────────────────────────────────────

    fn kanagawa() -> Self {
        Self {
            id: ThemeId::Kanagawa,
            bg_dark: Color::Rgb(22, 22, 29),
            bg_panel: Color::Rgb(31, 31, 40),
            border_dim: Color::Rgb(84, 84, 109),
            border_glow: Color::Rgb(126, 156, 216),
            text_dim: Color::Rgb(84, 84, 109),
            text_bright: Color::Rgb(220, 215, 186),
            accent_primary: Color::Rgb(126, 156, 216),    // crystal blue
            accent_secondary: Color::Rgb(149, 127, 184),  // ono violet
            accent_tertiary: Color::Rgb(127, 180, 202),   // spring blue
            accent_warning: Color::Rgb(220, 165, 100),    // ronin yellow
            accent_error: Color::Rgb(195, 64, 67),        // autumn red
            accent_success: Color::Rgb(118, 148, 106),    // spring green
            gradient: [
                Color::Rgb(118, 148, 106),
                Color::Rgb(127, 180, 202),
                Color::Rgb(126, 156, 216),
                Color::Rgb(149, 127, 184),
            ],
            sparkline_fill: Color::Rgb(49, 52, 68),
            header_accent: Color::Rgb(126, 156, 216),
        }
    }

    // ── Dracula ──────────────────────────────────────────────────────────

    fn dracula() -> Self {
        Self {
            id: ThemeId::Dracula,
            bg_dark: Color::Rgb(21, 22, 30),
            bg_panel: Color::Rgb(40, 42, 54),
            border_dim: Color::Rgb(68, 71, 90),
            border_glow: Color::Rgb(189, 147, 249),
            text_dim: Color::Rgb(98, 114, 164),
            text_bright: Color::Rgb(248, 248, 242),
            accent_primary: Color::Rgb(189, 147, 249),    // purple
            accent_secondary: Color::Rgb(255, 121, 198),  // pink
            accent_tertiary: Color::Rgb(139, 233, 253),   // cyan
            accent_warning: Color::Rgb(241, 250, 140),    // yellow
            accent_error: Color::Rgb(255, 85, 85),        // red
            accent_success: Color::Rgb(80, 250, 123),     // green
            gradient: [
                Color::Rgb(80, 250, 123),
                Color::Rgb(139, 233, 253),
                Color::Rgb(189, 147, 249),
                Color::Rgb(255, 121, 198),
            ],
            sparkline_fill: Color::Rgb(53, 55, 70),
            header_accent: Color::Rgb(189, 147, 249),
        }
    }

    // ── Everforest ───────────────────────────────────────────────────────

    fn everforest() -> Self {
        Self {
            id: ThemeId::Everforest,
            bg_dark: Color::Rgb(29, 35, 34),
            bg_panel: Color::Rgb(38, 45, 43),
            border_dim: Color::Rgb(78, 91, 88),
            border_glow: Color::Rgb(131, 192, 146),
            text_dim: Color::Rgb(100, 116, 113),
            text_bright: Color::Rgb(211, 198, 170),
            accent_primary: Color::Rgb(131, 192, 146),    // aqua
            accent_secondary: Color::Rgb(167, 192, 128),  // green
            accent_tertiary: Color::Rgb(125, 196, 228),   // blue
            accent_warning: Color::Rgb(219, 188, 127),    // yellow
            accent_error: Color::Rgb(230, 126, 128),      // red
            accent_success: Color::Rgb(167, 192, 128),    // green
            gradient: [
                Color::Rgb(167, 192, 128),
                Color::Rgb(131, 192, 146),
                Color::Rgb(125, 196, 228),
                Color::Rgb(219, 188, 127),
            ],
            sparkline_fill: Color::Rgb(56, 68, 66),
            header_accent: Color::Rgb(167, 192, 128),
        }
    }

    // ── One Dark ─────────────────────────────────────────────────────────

    fn one_dark() -> Self {
        Self {
            id: ThemeId::OneDark,
            bg_dark: Color::Rgb(24, 26, 31),
            bg_panel: Color::Rgb(40, 44, 52),
            border_dim: Color::Rgb(59, 66, 82),
            border_glow: Color::Rgb(97, 175, 239),
            text_dim: Color::Rgb(92, 99, 112),
            text_bright: Color::Rgb(171, 178, 191),
            accent_primary: Color::Rgb(97, 175, 239),     // blue
            accent_secondary: Color::Rgb(198, 120, 221),  // purple
            accent_tertiary: Color::Rgb(86, 182, 194),    // cyan
            accent_warning: Color::Rgb(229, 192, 123),    // yellow
            accent_error: Color::Rgb(224, 108, 117),      // red
            accent_success: Color::Rgb(152, 195, 121),    // green
            gradient: [
                Color::Rgb(152, 195, 121),
                Color::Rgb(86, 182, 194),
                Color::Rgb(97, 175, 239),
                Color::Rgb(198, 120, 221),
            ],
            sparkline_fill: Color::Rgb(60, 68, 80),
            header_accent: Color::Rgb(97, 175, 239),
        }
    }

    // ── Moonfly ──────────────────────────────────────────────────────────

    fn moonfly() -> Self {
        Self {
            id: ThemeId::Moonfly,
            bg_dark: Color::Rgb(15, 15, 22),
            bg_panel: Color::Rgb(27, 29, 36),
            border_dim: Color::Rgb(58, 61, 76),
            border_glow: Color::Rgb(130, 194, 255),
            text_dim: Color::Rgb(78, 82, 103),
            text_bright: Color::Rgb(188, 197, 220),
            accent_primary: Color::Rgb(130, 194, 255),    // cornflower blue
            accent_secondary: Color::Rgb(160, 140, 255),  // violet
            accent_tertiary: Color::Rgb(115, 218, 202),   // turquoise
            accent_warning: Color::Rgb(255, 203, 107),    // khaki
            accent_error: Color::Rgb(255, 92, 87),        // red
            accent_success: Color::Rgb(166, 218, 149),    // mint green
            gradient: [
                Color::Rgb(166, 218, 149),
                Color::Rgb(115, 218, 202),
                Color::Rgb(130, 194, 255),
                Color::Rgb(160, 140, 255),
            ],
            sparkline_fill: Color::Rgb(40, 42, 55),
            header_accent: Color::Rgb(130, 194, 255),
        }
    }

    // ── Style helpers ────────────────────────────────────────────────────

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border_dim)
    }

    pub fn glow_border_style(&self, phase: f64) -> Style {
        let glow = crate::ui::animation::breathing(phase);
        let dim = self.border_dim;
        let bright = self.border_glow;
        let color = lerp_color(dim, bright, glow);
        Style::default().fg(color)
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
