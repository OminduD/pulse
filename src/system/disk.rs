//! Disk data collection: per-mount usage and I/O throughput.

use sysinfo::Disks;

/// Per-mount-point disk usage.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct DiskInfo {
    pub name: String,
    pub mount: String,
    pub used: u64,
    pub total: u64,
    pub fs: String,
}

impl DiskInfo {
    pub fn usage_ratio(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.used as f64 / self.total as f64
        }
    }
}

/// Global disk I/O throughput snapshot.
#[derive(Clone, Debug, Default)]
pub struct DiskIoSnapshot {
    /// Bytes read per second.
    pub read_speed: f64,
    /// Bytes written per second.
    pub write_speed: f64,
    /// Total bytes read since boot.
    pub total_read: u64,
    /// Total bytes written since boot.
    pub total_write: u64,
    /// IO wait percentage (from /proc/stat).
    pub io_wait_pct: f64,
}

pub fn collect_usage(disks: &Disks) -> Vec<DiskInfo> {
    disks
        .iter()
        .map(|d| {
            let total = d.total_space();
            let available = d.available_space();
            DiskInfo {
                name: d.name().to_string_lossy().to_string(),
                mount: d.mount_point().to_string_lossy().to_string(),
                used: total.saturating_sub(available),
                total,
                fs: String::from_utf8_lossy(d.file_system().as_encoded_bytes()).to_string(),
            }
        })
        .collect()
}

/// Collect disk I/O from /proc/diskstats (Linux).
pub fn collect_io(prev_read: &mut u64, prev_write: &mut u64, dt_secs: f64) -> DiskIoSnapshot {
    let (total_read, total_write) = read_diskstats();
    let io_wait_pct = read_io_wait();

    let read_delta = total_read.saturating_sub(*prev_read);
    let write_delta = total_write.saturating_sub(*prev_write);

    let dt = if dt_secs > 0.0 { dt_secs } else { 1.0 };
    let read_speed = read_delta as f64 / dt;
    let write_speed = write_delta as f64 / dt;

    *prev_read = total_read;
    *prev_write = total_write;

    DiskIoSnapshot {
        read_speed,
        write_speed,
        total_read,
        total_write,
        io_wait_pct,
    }
}

/// Parse /proc/diskstats to sum sector reads/writes across physical disks.
/// Each sector is 512 bytes.
fn read_diskstats() -> (u64, u64) {
    let mut total_read = 0u64;
    let mut total_write = 0u64;

    if let Ok(contents) = std::fs::read_to_string("/proc/diskstats") {
        for line in contents.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 10 {
                let name = fields[2];
                // Only count physical disks (sd*, nvme*, vd*), not partitions
                let is_disk = (name.starts_with("sd") && name.len() == 3)
                    || (name.starts_with("nvme") && name.contains("n") && !name.contains("p"))
                    || (name.starts_with("vd") && name.len() == 3);
                if is_disk {
                    if let Ok(sectors_read) = fields[5].parse::<u64>() {
                        total_read += sectors_read * 512;
                    }
                    if let Ok(sectors_written) = fields[9].parse::<u64>() {
                        total_write += sectors_written * 512;
                    }
                }
            }
        }
    }
    (total_read, total_write)
}

/// Read IO wait from /proc/stat (4th field of the cpu line).
fn read_io_wait() -> f64 {
    if let Ok(contents) = std::fs::read_to_string("/proc/stat") {
        if let Some(cpu_line) = contents.lines().next() {
            let fields: Vec<&str> = cpu_line.split_whitespace().collect();
            // cpu user nice system idle iowait irq softirq steal
            if fields.len() >= 6 && fields[0] == "cpu" {
                let _idle: f64 = fields[4].parse().unwrap_or(0.0);
                let iowait: f64 = fields[5].parse().unwrap_or(0.0);
                let total: f64 = fields[1..]
                    .iter()
                    .filter_map(|f| f.parse::<f64>().ok())
                    .sum();
                if total > 0.0 {
                    return (iowait / total) * 100.0;
                }
            }
        }
    }
    0.0
}
