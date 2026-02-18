//! Animation helpers: pulsing, scrolling patterns, glow effects, braille sparklines,
//! rainbow cycling, wave effects, spinners, breathing borders, and particle systems.

#![allow(dead_code)]

// ── Basic pulses ─────────────────────────────────────────────────────────────

/// Compute a smooth sine-wave pulse value (0.0–1.0) from a phase (0.0–1.0).
pub fn pulse_value(phase: f64) -> f64 {
    (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5
}

/// Compute a faster double-frequency pulse.
pub fn fast_pulse(phase: f64) -> f64 {
    (phase * std::f64::consts::TAU * 2.0).sin() * 0.5 + 0.5
}

/// Smooth ease-in-out pulse — feels organic and bouncy.
pub fn ease_pulse(phase: f64) -> f64 {
    let t = (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5;
    t * t * (3.0 - 2.0 * t)
}

/// A breathing effect — slow ramp up + hold + ramp down.
pub fn breathing(phase: f64) -> f64 {
    let t = (phase * std::f64::consts::TAU).sin();
    (t * 0.5 + 0.5).powf(0.8)
}

// ── Pattern generators ───────────────────────────────────────────────────────

/// Generate a scrolling decorative pattern string of the given width.
pub fn scrolling_pattern(tick: u64, width: usize) -> String {
    let pattern: Vec<char> = "░▒▓█▓▒░".chars().collect();
    let offset = (tick as usize) % pattern.len();
    (0..width)
        .map(|i| pattern[(i + offset) % pattern.len()])
        .collect()
}

/// Generate a wave pattern using braille dots that scrolls horizontally.
pub fn wave_pattern(tick: u64, width: usize) -> String {
    let chars = ['⠁', '⠂', '⠄', '⠂', '⠁', '⠈', '⠐', '⠈'];
    let offset = tick as usize;
    (0..width)
        .map(|i| chars[(i + offset) % chars.len()])
        .collect()
}

/// Cyberpunk scanning line pattern.
pub fn scan_line(tick: u64, width: usize) -> String {
    let pos = (tick as usize * 2) % width;
    let glow_radius = 4;
    (0..width)
        .map(|i| {
            let dist = (i as isize - pos as isize).unsigned_abs();
            if dist == 0 {
                '█'
            } else if dist <= glow_radius {
                match dist {
                    1 => '▓',
                    2 => '▒',
                    3 => '░',
                    _ => '·',
                }
            } else {
                ' '
            }
        })
        .collect()
}

/// Generate a matrix-style column of random characters.
pub fn matrix_column(height: usize, tick: u64, col: usize) -> Vec<char> {
    let chars = "ｱｲｳｴｵｶｷｸｹｺ01";
    let chars_vec: Vec<char> = chars.chars().collect();
    let len = chars_vec.len();

    (0..height)
        .map(|row| {
            let seed = (tick as usize)
                .wrapping_mul(7)
                .wrapping_add(col * 13)
                .wrapping_add(row * 31);
            chars_vec[seed % len]
        })
        .collect()
}

// ── Glow & intensity effects ─────────────────────────────────────────────────

/// Glow intensity based on a value and phase, for highlighting top processes.
/// Returns a brightness modifier 0.0–1.0.
pub fn glow_intensity(value: f64, threshold: f64, phase: f64) -> f64 {
    if value > threshold {
        let excess = ((value - threshold) / threshold).clamp(0.0, 1.0);
        let glow = pulse_value(phase) * 0.3 + 0.7;
        excess * glow
    } else {
        0.0
    }
}

/// Flicker effect for critical alerts — returns true/false for visibility toggle.
pub fn flicker(tick: u64, rate: u64) -> bool {
    (tick / rate) % 2 == 0
}

// ── Progress bars ────────────────────────────────────────────────────────────

/// Progress bar with animated fill shimmer effect.
pub fn shimmer_bar(ratio: f64, width: usize, tick: u64) -> String {
    let filled = (ratio.clamp(0.0, 1.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    if filled == 0 {
        return "░".repeat(width);
    }

    let shimmer_pos = (tick as usize) % filled.max(1);
    let mut bar = String::with_capacity(width * 3);

    for i in 0..filled {
        if i == shimmer_pos {
            bar.push('▓');
        } else {
            bar.push('█');
        }
    }
    for _ in 0..empty {
        bar.push('░');
    }
    bar
}

/// Gradient bar using Unicode fractional blocks with shimmer.
pub fn gradient_bar(ratio: f64, width: usize, tick: u64) -> String {
    let filled_f = ratio.clamp(0.0, 1.0) * width as f64;
    let filled = filled_f as usize;
    let frac = filled_f - filled as f64;
    let shimmer = (tick as usize) % width;

    let mut bar = String::with_capacity(width * 4);
    let blocks = ['░', '▒', '▓', '█'];

    for i in 0..width {
        if i < filled {
            if i == shimmer {
                bar.push('▓');
            } else {
                bar.push('█');
            }
        } else if i == filled {
            let idx = (frac * 3.0) as usize;
            bar.push(blocks[idx.min(3)]);
        } else {
            bar.push(' ');
        }
    }
    bar
}

/// Flame-style usage bar characters.
pub fn flame_bar(ratio: f64, width: usize) -> String {
    let flames = ['░', '▒', '▓', '█'];
    let filled = (ratio.clamp(0.0, 1.0) * width as f64).round() as usize;

    let mut bar = String::new();
    for i in 0..width {
        if i < filled {
            let intensity = i as f64 / width as f64;
            let idx = (intensity * 3.0) as usize;
            bar.push(flames[idx.min(3)]);
        } else {
            bar.push(' ');
        }
    }
    bar
}

// ── Braille sparkline ────────────────────────────────────────────────────────

/// Higher resolution sparklines using braille characters.
/// Returns a string of braille characters representing the data.
pub fn braille_sparkline(data: &[f64], max: f64, width: usize) -> String {
    let braille_chars = [' ', '⣀', '⣤', '⣶', '⣿'];
    let m = if max <= 0.0 { 1.0 } else { max };

    let start = if data.len() > width {
        data.len() - width
    } else {
        0
    };
    let slice = &data[start..];

    let mut result = String::with_capacity(width * 3);
    for i in 0..width {
        if i < slice.len() {
            let ratio = (slice[i] / m).clamp(0.0, 1.0);
            let idx = (ratio * 4.0) as usize;
            result.push(braille_chars[idx.min(4)]);
        } else {
            result.push(' ');
        }
    }
    result
}

// ── Spinners ─────────────────────────────────────────────────────────────────

/// Rotating spinner character.
pub fn spinner(tick: u64) -> char {
    let frames = ['◐', '◓', '◑', '◒'];
    frames[(tick as usize / 3) % frames.len()]
}

/// Dot spinner for loading indicators.
pub fn dot_spinner(tick: u64) -> &'static str {
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    frames[(tick as usize / 2) % frames.len()]
}

/// Bouncing bar spinner.
pub fn bounce_bar(tick: u64, width: usize) -> String {
    let cycle = width * 2;
    let pos = (tick as usize) % cycle;
    let actual = if pos < width { pos } else { cycle - pos };
    let mut bar = vec![' '; width];
    if actual < width {
        bar[actual] = '█';
        if actual > 0 {
            bar[actual - 1] = '▒';
        }
        if actual + 1 < width {
            bar[actual + 1] = '▒';
        }
    }
    bar.into_iter().collect()
}

// ── Rainbow / color cycling ──────────────────────────────────────────────────

/// Generate RGB values for a rainbow cycle. Returns (r, g, b).
pub fn rainbow_rgb(phase: f64, offset: f64) -> (u8, u8, u8) {
    let t = ((phase + offset) % 1.0) * 6.0;
    let segment = t as u8;
    let frac = t - segment as f64;

    match segment % 6 {
        0 => (255, (frac * 255.0) as u8, 0),
        1 => ((255.0 * (1.0 - frac)) as u8, 255, 0),
        2 => (0, 255, (frac * 255.0) as u8),
        3 => (0, (255.0 * (1.0 - frac)) as u8, 255),
        4 => ((frac * 255.0) as u8, 0, 255),
        _ => (255, 0, (255.0 * (1.0 - frac)) as u8),
    }
}

/// Generate a neon glow color that cycles between accent tones.
pub fn neon_cycle(phase: f64) -> (u8, u8, u8) {
    let t = (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5;
    let r = (t * 200.0) as u8;
    let g = ((1.0 - t) * 255.0) as u8;
    let b = 255;
    (r, g, b)
}

// ── Border effects ───────────────────────────────────────────────────────────

/// Returns a border glow color that pulses based on the activity level.
pub fn border_glow_color(activity: f64, phase: f64) -> (u8, u8, u8) {
    let glow = pulse_value(phase * (1.0 + activity * 2.0));
    let base_brightness = 40.0 + activity * 60.0;
    let pulse_brightness = glow * 40.0;
    let total = (base_brightness + pulse_brightness).min(255.0) as u8;

    if activity > 0.8 {
        (total, (total as f64 * 0.4) as u8, 0)
    } else if activity > 0.5 {
        (0, total, total)
    } else {
        ((total as f64 * 0.4) as u8, (total as f64 * 0.3) as u8, total)
    }
}

// ── Activity meters ──────────────────────────────────────────────────────────

/// Generate an activity indicator: animated bars that respond to value.
pub fn activity_indicator(value: f64, width: usize, tick: u64) -> String {
    let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let v = value.clamp(0.0, 1.0);

    (0..width)
        .map(|i| {
            let wave = ((i as f64 / width as f64 + tick as f64 / 20.0) * std::f64::consts::TAU)
                .sin()
                * 0.15
                + 0.85;
            let level = (v * wave * 7.0) as usize;
            blocks[level.min(7)]
        })
        .collect()
}

/// Animated CPU core display using vertical bars.
pub fn core_activity_bars(values: &[f64], tick: u64) -> Vec<(String, f64)> {
    let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let ratio = (v / 100.0).clamp(0.0, 1.0);
            let wave = ((i as f64 * 0.5 + tick as f64 / 30.0) * std::f64::consts::TAU)
                .sin()
                * 0.05;
            let animated = (ratio + wave).clamp(0.0, 1.0);
            let idx = (animated * 7.0) as usize;
            (blocks[idx.min(7)].to_string(), ratio)
        })
        .collect()
}

// ── Header / title effects ───────────────────────────────────────────────────

/// Generate a glitch effect on a string — randomly shifts characters.
pub fn glitch_text(text: &str, tick: u64, intensity: f64) -> String {
    let glitch_chars = ['#', '@', '!', '%', '&', '/', '?', '~', '*'];
    let hash_base = tick.wrapping_mul(2654435761);

    text.chars()
        .enumerate()
        .map(|(i, c)| {
            let h = hash_base.wrapping_add(i as u64 * 37);
            let should_glitch = (h % 100) < (intensity * 10.0) as u64;
            if should_glitch && c != ' ' {
                glitch_chars[(h as usize / 7) % glitch_chars.len()]
            } else {
                c
            }
        })
        .collect()
}
