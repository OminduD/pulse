//! Memory data collection: used, free, cached, buffers, swap.

use sysinfo::System;

/// Memory and swap usage.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MemorySnapshot {
    pub used: u64,
    pub free: u64,
    pub total: u64,
    /// Cached memory (from /proc/meminfo on Linux).
    pub cached: u64,
    /// Buffer memory.
    pub buffers: u64,
    pub swap_used: u64,
    pub swap_total: u64,
}

impl MemorySnapshot {
    /// Usage ratio 0.0–1.0.
    pub fn usage_ratio(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.used as f64 / self.total as f64
        }
    }

    /// Swap usage ratio 0.0–1.0.
    pub fn swap_ratio(&self) -> f64 {
        if self.swap_total == 0 {
            0.0
        } else {
            self.swap_used as f64 / self.swap_total as f64
        }
    }
}

pub fn collect(sys: &System) -> MemorySnapshot {
    let total = sys.total_memory();
    let used = sys.used_memory();
    let free = sys.free_memory();
    let swap_used = sys.used_swap();
    let swap_total = sys.total_swap();

    // Try to read cached/buffers from /proc/meminfo
    let (cached, buffers) = read_cache_buffers();

    MemorySnapshot {
        used,
        free,
        total,
        cached,
        buffers,
        swap_used,
        swap_total,
    }
}

/// Parse /proc/meminfo for Cached and Buffers values.
fn read_cache_buffers() -> (u64, u64) {
    let mut cached = 0u64;
    let mut buffers = 0u64;

    if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
        for line in contents.lines() {
            if let Some(val) = line.strip_prefix("Cached:") {
                cached = parse_meminfo_kb(val);
            } else if let Some(val) = line.strip_prefix("Buffers:") {
                buffers = parse_meminfo_kb(val);
            }
        }
    }
    (cached, buffers)
}

/// Parse a /proc/meminfo value like "   12345 kB" → bytes.
fn parse_meminfo_kb(s: &str) -> u64 {
    s.trim()
        .split_whitespace()
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
        * 1024 // kB → bytes
}
