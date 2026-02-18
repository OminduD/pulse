//! System data collection. Wraps `sysinfo` and produces snapshot structs
//! that the UI layer can render without touching sysinfo directly.

use sysinfo::{Disks, Networks, System};

// ── Snapshot types ───────────────────────────────────────────────────────────

/// Per-core CPU usage (0.0–100.0).
#[derive(Clone, Debug)]
pub struct CpuSnapshot {
    pub per_core: Vec<f64>,
    pub global: f64,
}

/// Memory & swap usage in bytes.
#[derive(Clone, Debug)]
pub struct MemorySnapshot {
    pub used: u64,
    pub total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
}

/// Single disk entry.
#[derive(Clone, Debug)]
pub struct DiskInfo {
    pub name: String,
    pub mount: String,
    pub used: u64,
    pub total: u64,
    pub fs: String,
}

/// Network traffic accumulated since last refresh.
#[derive(Clone, Debug, Default)]
pub struct NetSnapshot {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    /// Bytes/sec received (computed from delta).
    pub rx_speed: f64,
    /// Bytes/sec transmitted.
    pub tx_speed: f64,
}

/// Single process entry.
#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub mem_mb: f64,
    pub status: String,
}

// ── Collector ────────────────────────────────────────────────────────────────

/// Holds all sysinfo handles and produces snapshots on demand.
pub struct SystemCollector {
    sys: System,
    disks: Disks,
    networks: Networks,
    prev_rx: u64,
    prev_tx: u64,
}

impl SystemCollector {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        Self {
            sys,
            disks,
            networks,
            prev_rx: 0,
            prev_tx: 0,
        }
    }

    /// Refresh all data sources. Call this every ~500 ms.
    pub fn refresh(&mut self) {
        self.sys.refresh_all();
        self.disks.refresh();
        self.networks.refresh();
    }

    // ── Snapshots ────────────────────────────────────────────────────────

    pub fn cpu(&self) -> CpuSnapshot {
        let per_core: Vec<f64> = self.sys.cpus().iter().map(|c| c.cpu_usage() as f64).collect();
        let global = self.sys.global_cpu_usage() as f64;
        CpuSnapshot { per_core, global }
    }

    pub fn memory(&self) -> MemorySnapshot {
        MemorySnapshot {
            used: self.sys.used_memory(),
            total: self.sys.total_memory(),
            swap_used: self.sys.used_swap(),
            swap_total: self.sys.total_swap(),
        }
    }

    pub fn disks(&self) -> Vec<DiskInfo> {
        self.disks
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

    /// Returns network snapshot and computes speed from delta.
    pub fn network(&mut self, dt_secs: f64) -> NetSnapshot {
        let (rx, tx) = self
            .networks
            .iter()
            .fold((0u64, 0u64), |(r, t), (_name, data)| {
                (r + data.total_received(), t + data.total_transmitted())
            });

        let rx_delta = rx.saturating_sub(self.prev_rx);
        let tx_delta = tx.saturating_sub(self.prev_tx);
        self.prev_rx = rx;
        self.prev_tx = tx;

        let dt = if dt_secs > 0.0 { dt_secs } else { 1.0 };
        NetSnapshot {
            rx_bytes: rx,
            tx_bytes: tx,
            rx_speed: rx_delta as f64 / dt,
            tx_speed: tx_delta as f64 / dt,
        }
    }

    pub fn processes(&self) -> Vec<ProcessInfo> {
        self.sys
            .processes()
            .values()
            .map(|p| ProcessInfo {
                pid: p.pid().as_u32(),
                name: p.name().to_string_lossy().to_string(),
                cpu: p.cpu_usage(),
                mem_mb: p.memory() as f64 / 1_048_576.0,
                status: format!("{:?}", p.status()),
            })
            .collect()
    }
}
