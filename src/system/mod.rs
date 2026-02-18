//! System data collection modules.
//!
//! Each submodule owns its snapshot types and collection logic.
//! The top-level [`SystemCollector`] aggregates them into a single refresh cycle.

pub mod cpu;
pub mod disk;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod process;

use sysinfo::{Disks, Networks, System};

use cpu::CpuSnapshot;
use disk::{DiskInfo, DiskIoSnapshot};
use memory::MemorySnapshot;
use network::{InterfaceStats, NetSnapshot};
use process::ProcessInfo;

// ── Unified collector ────────────────────────────────────────────────────────

/// Aggregates all system data sources and produces snapshots on demand.
pub struct SystemCollector {
    sys: System,
    disks: Disks,
    networks: Networks,
    prev_rx: u64,
    prev_tx: u64,
    prev_disk_read: u64,
    prev_disk_write: u64,
    /// Per-interface previous counters for per-if speed calculation.
    prev_if_rx: std::collections::HashMap<String, u64>,
    prev_if_tx: std::collections::HashMap<String, u64>,
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
            prev_disk_read: 0,
            prev_disk_write: 0,
            prev_if_rx: std::collections::HashMap::new(),
            prev_if_tx: std::collections::HashMap::new(),
        }
    }

    /// Refresh all data sources. Call every ~500 ms.
    pub fn refresh(&mut self) {
        self.sys.refresh_all();
        self.disks.refresh();
        self.networks.refresh();
    }

    pub fn cpu(&self) -> CpuSnapshot {
        cpu::collect(&self.sys)
    }

    pub fn memory(&self) -> MemorySnapshot {
        memory::collect(&self.sys)
    }

    pub fn disks(&self) -> Vec<DiskInfo> {
        disk::collect_usage(&self.disks)
    }

    pub fn disk_io(&mut self, dt_secs: f64) -> DiskIoSnapshot {
        disk::collect_io(&mut self.prev_disk_read, &mut self.prev_disk_write, dt_secs)
    }

    pub fn network(&mut self, dt_secs: f64) -> NetSnapshot {
        network::collect_aggregate(
            &self.networks,
            &mut self.prev_rx,
            &mut self.prev_tx,
            dt_secs,
        )
    }

    pub fn per_interface(&mut self, dt_secs: f64) -> Vec<InterfaceStats> {
        network::collect_per_interface(
            &self.networks,
            &mut self.prev_if_rx,
            &mut self.prev_if_tx,
            dt_secs,
        )
    }

    pub fn processes(&self) -> Vec<ProcessInfo> {
        process::collect(&self.sys)
    }

    pub fn uptime(&self) -> u64 {
        System::uptime()
    }

    pub fn num_cpus(&self) -> usize {
        self.sys.cpus().len()
    }
}
