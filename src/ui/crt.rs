//! CRT post-processing effects. Operates on the ratatui Buffer after all
//! widgets have been rendered, simulating a retro CRT monitor look.
//!
//! Effects:
//! - **Scanlines**: Dim every other row to simulate CRT phosphor gaps.
//! - **Vignette**: Darken corners and edges to simulate curved CRT glass.
//! - **Chromatic Aberration**: Shift red/blue channels on high-contrast text.
//! - **Phosphor Glow**: Add a faint green/amber tint to bright cells.
//! - **Noise**: Random speckle to simulate analog signal interference.

use ratatui::buffer::Buffer;
use ratatui::style::Color;

/// CRT configuration parameters (from config).
#[derive(Debug, Clone)]
pub struct CrtConfig {
    pub scanline_intensity: f64,
    pub vignette_intensity: f64,
    pub aberration: f64,
    pub glow: f64,
}

impl Default for CrtConfig {
    fn default() -> Self {
        Self {
            scanline_intensity: 0.3,
            vignette_intensity: 0.4,
            aberration: 0.2,
            glow: 0.15,
        }
    }
}

/// Apply all CRT effects to a rendered buffer.
pub fn apply_crt_effects(buf: &mut Buffer, config: &CrtConfig, tick: u64) {
    let area = buf.area;
    if area.width == 0 || area.height == 0 {
        return;
    }

    // Apply effects in order: scanlines → vignette → aberration → glow → noise
    if config.scanline_intensity > 0.01 {
        apply_scanlines(buf, config.scanline_intensity, tick);
    }
    if config.vignette_intensity > 0.01 {
        apply_vignette(buf, config.vignette_intensity);
    }
    if config.aberration > 0.01 {
        apply_aberration(buf, config.aberration);
    }
    if config.glow > 0.01 {
        apply_phosphor_glow(buf, config.glow);
    }
    // Noise is subtle and tick-dependent
    apply_noise(buf, 0.02, tick);
}

// ── Scanlines ────────────────────────────────────────────────────────────────

/// Dim every other row to simulate CRT scanline gaps.
/// `intensity` controls how much the rows are dimmed (0.0 = none, 1.0 = black).
fn apply_scanlines(buf: &mut Buffer, intensity: f64, tick: u64) {
    let area = buf.area;
    // Slight roll effect: shift which rows are dimmed over time
    let offset = (tick / 4) as u16 % 2;

    for y in area.y..area.y + area.height {
        if (y + offset) % 2 == 1 {
            for x in area.x..area.x + area.width {
                let cell = &mut buf[(x, y)];
                cell.set_fg(dim_color(cell.fg, intensity * 0.6));
                cell.set_bg(dim_color(cell.bg, intensity));
            }
        }
    }
}

// ── Vignette ─────────────────────────────────────────────────────────────────

/// Darken the edges and corners of the screen, simulating CRT curvature.
fn apply_vignette(buf: &mut Buffer, intensity: f64) {
    let area = buf.area;
    let cx = area.width as f64 / 2.0;
    let cy = area.height as f64 / 2.0;
    let max_dist = (cx * cx + cy * cy).sqrt();

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let dx = (x as f64 - cx) / cx;
            let dy = (y as f64 - cy) / cy;
            let dist = (dx * dx + dy * dy).sqrt();

            // Smooth vignette falloff starting at ~60% from center
            let vignette = ((dist - 0.6) / 0.8).clamp(0.0, 1.0) * intensity;

            if vignette > 0.01 {
                let cell = &mut buf[(x, y)];
                cell.set_fg(dim_color(cell.fg, vignette));
                cell.set_bg(dim_color(cell.bg, vignette));
            }
        }
    }
    let _ = max_dist; // suppress unused
}

// ── Chromatic Aberration ─────────────────────────────────────────────────────

/// Simulate chromatic aberration by shifting red channel left and blue right
/// for bright, high-contrast cells.
fn apply_aberration(buf: &mut Buffer, strength: f64) {
    let area = buf.area;
    if area.width < 3 {
        return;
    }

    // We need to read neighbouring cells, so work on a snapshot of the colors
    let width = area.width as usize;
    let height = area.height as usize;

    // Collect original fg colors in row-major order
    let mut fg_colors: Vec<(u8, u8, u8)> = Vec::with_capacity(width * height);
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = &buf[(x, y)];
            fg_colors.push(color_to_rgb(cell.fg));
        }
    }

    let shift = (strength * 1.5).max(1.0) as usize;

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let (r, g, b) = fg_colors[idx];

            // Only apply to non-dim cells (brightness > threshold)
            let brightness = (r as u16 + g as u16 + b as u16) / 3;
            if brightness > 80 {
                // Red shifted from left, blue shifted from right
                let r_src = if x >= shift { fg_colors[y * width + (x - shift)].0 } else { r };
                let b_src = if x + shift < width { fg_colors[y * width + (x + shift)].2 } else { b };

                let mix = (strength * 0.6).clamp(0.0, 0.5);
                let nr = lerp_u8(r, r_src, mix);
                let nb = lerp_u8(b, b_src, mix);

                let cell = &mut buf[(x as u16 + area.x, y as u16 + area.y)];
                cell.set_fg(Color::Rgb(nr, g, nb));
            }
        }
    }
}

// ── Phosphor Glow ────────────────────────────────────────────────────────────

/// Add a faint phosphor tint to bright cells, simulating CRT phosphor glow.
/// Uses a warm amber/green tint.
fn apply_phosphor_glow(buf: &mut Buffer, intensity: f64) {
    let area = buf.area;
    let glow_r: u8 = 20;
    let glow_g: u8 = 40;
    let glow_b: u8 = 15;

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = &mut buf[(x, y)];
            let (r, g, b) = color_to_rgb(cell.fg);
            let brightness = (r as u16 + g as u16 + b as u16) / 3;

            if brightness > 100 {
                let glow_amount = ((brightness as f64 - 100.0) / 155.0) * intensity;
                let nr = (r as f64 + glow_r as f64 * glow_amount).min(255.0) as u8;
                let ng = (g as f64 + glow_g as f64 * glow_amount).min(255.0) as u8;
                let nb = (b as f64 + glow_b as f64 * glow_amount).min(255.0) as u8;
                cell.set_fg(Color::Rgb(nr, ng, nb));
            }
        }
    }
}

// ── Noise ────────────────────────────────────────────────────────────────────

/// Add subtle random noise to simulate analog signal interference.
fn apply_noise(buf: &mut Buffer, intensity: f64, tick: u64) {
    let area = buf.area;
    let hash_base = tick.wrapping_mul(2654435761);

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            // Cheap pseudo-random
            let h = hash_base
                .wrapping_add((x as u64).wrapping_mul(73))
                .wrapping_add((y as u64).wrapping_mul(137));

            // Only affect ~5% of cells per frame
            if h % 20 == 0 {
                let cell = &mut buf[(x, y)];
                let (r, g, b) = color_to_rgb(cell.fg);
                let noise = ((h % 30) as f64 - 15.0) * intensity;
                let nr = (r as f64 + noise).clamp(0.0, 255.0) as u8;
                let ng = (g as f64 + noise).clamp(0.0, 255.0) as u8;
                let nb = (b as f64 + noise).clamp(0.0, 255.0) as u8;
                cell.set_fg(Color::Rgb(nr, ng, nb));
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Dim a color by `amount` (0.0 = unchanged, 1.0 = black).
fn dim_color(color: Color, amount: f64) -> Color {
    let (r, g, b) = color_to_rgb(color);
    let factor = (1.0 - amount).clamp(0.0, 1.0);
    Color::Rgb(
        (r as f64 * factor) as u8,
        (g as f64 * factor) as u8,
        (b as f64 * factor) as u8,
    )
}

/// Extract RGB values from a Color, defaulting non-RGB variants.
fn color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Reset => (0, 0, 0),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 0, 0),
        Color::Green => (0, 205, 0),
        Color::Yellow => (205, 205, 0),
        Color::Blue => (0, 0, 238),
        Color::Magenta => (205, 0, 205),
        Color::Cyan => (0, 205, 205),
        Color::Gray => (128, 128, 128),
        Color::DarkGray => (85, 85, 85),
        Color::LightRed => (255, 85, 85),
        Color::LightGreen => (85, 255, 85),
        Color::LightYellow => (255, 255, 85),
        Color::LightBlue => (85, 85, 255),
        Color::LightMagenta => (255, 85, 255),
        Color::LightCyan => (85, 255, 255),
        Color::White => (255, 255, 255),
        _ => (128, 128, 128),
    }
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t).round().clamp(0.0, 255.0) as u8
}
