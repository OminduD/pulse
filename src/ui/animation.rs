//! Animation helpers: pulsing, scrolling patterns, glow effects.

#![allow(dead_code)]

/// Compute a smooth sine-wave pulse value (0.0–1.0) from a phase (0.0–1.0).
pub fn pulse_value(phase: f64) -> f64 {
    (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5
}

/// Compute a faster double-frequency pulse.
pub fn fast_pulse(phase: f64) -> f64 {
    (phase * std::f64::consts::TAU * 2.0).sin() * 0.5 + 0.5
}

/// Generate a scrolling decorative pattern string of the given width.
pub fn scrolling_pattern(tick: u64, width: usize) -> String {
    let pattern: Vec<char> = "░▒▓█▓▒░".chars().collect();
    let offset = (tick as usize) % pattern.len();
    (0..width)
        .map(|i| pattern[(i + offset) % pattern.len()])
        .collect()
}

/// Generate a matrix-style column of random characters.
pub fn matrix_column(height: usize, tick: u64, col: usize) -> Vec<char> {
    let chars = "ｱｲｳｴｵｶｷｸｹｺ01";
    let chars_vec: Vec<char> = chars.chars().collect();
    let len = chars_vec.len();

    (0..height)
        .map(|row| {
            let seed = (tick as usize).wrapping_mul(7).wrapping_add(col * 13).wrapping_add(row * 31);
            chars_vec[seed % len]
        })
        .collect()
}

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

/// Progress bar with animated fill shimmer effect.
/// Returns a string where the fill boundary has a shimmer character.
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

/// Flame-style usage bar characters.
pub fn flame_bar(ratio: f64, width: usize) -> String {
    let flames = ['░', '▒', '▓', '█', '🔥'];
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
