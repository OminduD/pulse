//! CPU data collection: per-core usage, frequency, temperature, load averages.

use sysinfo::System;

/// Complete CPU data snapshot.
#[derive(Clone, Debug)]
pub struct CpuSnapshot {
    /// Per-core usage 0.0–100.0.
    pub per_core: Vec<f64>,
    /// Global (average) CPU usage.
    pub global: f64,
    /// Per-core frequency in MHz.
    pub frequencies: Vec<u64>,
    /// CPU temperature in °C (from first available sensor, if any).
    pub temperature: Option<f32>,
    /// Load averages: (1m, 5m, 15m). Only on Linux.
    pub load_avg: (f64, f64, f64),
}

pub fn collect(sys: &System) -> CpuSnapshot {
    let per_core: Vec<f64> = sys.cpus().iter().map(|c| c.cpu_usage() as f64).collect();
    let global = sys.global_cpu_usage() as f64;
    let frequencies: Vec<u64> = sys.cpus().iter().map(|c| c.frequency()).collect();

    // Temperature: read from /sys/class/thermal or /sys/class/hwmon
    let temperature = read_cpu_temperature();

    // Load averages from /proc/loadavg on Linux
    let load_avg = read_load_avg();

    CpuSnapshot {
        per_core,
        global,
        frequencies,
        temperature,
        load_avg,
    }
}

/// Read CPU temperature from Linux thermal zone sysfs.
fn read_cpu_temperature() -> Option<f32> {
    // Try /sys/class/thermal/thermal_zone0/temp first
    if let Ok(contents) = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
        if let Ok(millideg) = contents.trim().parse::<f32>() {
            return Some(millideg / 1000.0);
        }
    }
    // Fallback: try hwmon
    let hwmon_dir = std::path::Path::new("/sys/class/hwmon");
    if hwmon_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(hwmon_dir) {
            for entry in entries.flatten() {
                let temp_file = entry.path().join("temp1_input");
                if let Ok(contents) = std::fs::read_to_string(&temp_file) {
                    if let Ok(millideg) = contents.trim().parse::<f32>() {
                        return Some(millideg / 1000.0);
                    }
                }
            }
        }
    }
    None
}

/// Read load averages from /proc/loadavg.
fn read_load_avg() -> (f64, f64, f64) {
    if let Ok(contents) = std::fs::read_to_string("/proc/loadavg") {
        let parts: Vec<&str> = contents.split_whitespace().collect();
        if parts.len() >= 3 {
            let a = parts[0].parse().unwrap_or(0.0);
            let b = parts[1].parse().unwrap_or(0.0);
            let c = parts[2].parse().unwrap_or(0.0);
            return (a, b, c);
        }
    }
    (0.0, 0.0, 0.0)
}
