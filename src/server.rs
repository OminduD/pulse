//! Headless server mode. When launched with `--server`, Pulse collects
//! system metrics and emits one JSON line per refresh interval to stdout.
//! A remote Pulse client reads this stream over SSH.

use std::io::{self, Write};
use std::time::Duration;

use serde::Serialize;

use crate::system::cpu::CpuSnapshot;
use crate::system::disk::{DiskInfo, DiskIoSnapshot};
use crate::system::memory::MemorySnapshot;
use crate::system::network::NetSnapshot;
use crate::system::SystemCollector;

// ── Serialisable snapshots ───────────────────────────────────────────────────

/// A single JSON line emitted every refresh cycle.
#[derive(Serialize)]
pub struct ServerPacket {
    pub ts: i64,
    pub cpu: CpuData,
    pub mem: MemData,
    pub net: NetData,
    pub disks: Vec<DiskData>,
    pub disk_io: DiskIoData,
    pub uptime: u64,
}

#[derive(Serialize)]
pub struct CpuData {
    pub global: f64,
    pub per_core: Vec<f64>,
    pub frequencies: Vec<u64>,
    pub temperature: Option<f32>,
    pub load_avg: (f64, f64, f64),
}

#[derive(Serialize)]
pub struct MemData {
    pub used: u64,
    pub free: u64,
    pub total: u64,
    pub cached: u64,
    pub buffers: u64,
    pub swap_used: u64,
    pub swap_total: u64,
}

#[derive(Serialize)]
pub struct NetData {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_speed: f64,
    pub tx_speed: f64,
}

#[derive(Serialize)]
pub struct DiskData {
    pub name: String,
    pub mount: String,
    pub used: u64,
    pub total: u64,
    pub fs: String,
}

#[derive(Serialize)]
pub struct DiskIoData {
    pub read_speed: f64,
    pub write_speed: f64,
    pub total_read: u64,
    pub total_write: u64,
    pub io_wait_pct: f64,
}

// ── Conversion helpers ───────────────────────────────────────────────────────

impl From<&CpuSnapshot> for CpuData {
    fn from(s: &CpuSnapshot) -> Self {
        Self {
            global: s.global,
            per_core: s.per_core.clone(),
            frequencies: s.frequencies.clone(),
            temperature: s.temperature,
            load_avg: s.load_avg,
        }
    }
}

impl From<&MemorySnapshot> for MemData {
    fn from(s: &MemorySnapshot) -> Self {
        Self {
            used: s.used,
            free: s.free,
            total: s.total,
            cached: s.cached,
            buffers: s.buffers,
            swap_used: s.swap_used,
            swap_total: s.swap_total,
        }
    }
}

impl From<&NetSnapshot> for NetData {
    fn from(s: &NetSnapshot) -> Self {
        Self {
            rx_bytes: s.rx_bytes,
            tx_bytes: s.tx_bytes,
            rx_speed: s.rx_speed,
            tx_speed: s.tx_speed,
        }
    }
}

impl From<&DiskInfo> for DiskData {
    fn from(d: &DiskInfo) -> Self {
        Self {
            name: d.name.clone(),
            mount: d.mount.clone(),
            used: d.used,
            total: d.total,
            fs: d.fs.clone(),
        }
    }
}

impl From<&DiskIoSnapshot> for DiskIoData {
    fn from(s: &DiskIoSnapshot) -> Self {
        Self {
            read_speed: s.read_speed,
            write_speed: s.write_speed,
            total_read: s.total_read,
            total_write: s.total_write,
            io_wait_pct: s.io_wait_pct,
        }
    }
}

// ── Server loop ──────────────────────────────────────────────────────────────

/// Run the headless server loop. Emits one JSON line per `interval_ms` to stdout.
pub fn run_server(interval_ms: u64) -> color_eyre::Result<()> {
    let mut collector = SystemCollector::new();
    let dt = interval_ms as f64 / 1000.0;
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    loop {
        collector.refresh();

        let cpu = collector.cpu();
        let mem = collector.memory();
        let disks = collector.disks();
        let disk_io = collector.disk_io(dt.max(0.5));
        let net = collector.network(dt.max(0.5));
        let uptime = collector.uptime();

        let packet = ServerPacket {
            ts: chrono::Utc::now().timestamp(),
            cpu: CpuData::from(&cpu),
            mem: MemData::from(&mem),
            net: NetData::from(&net),
            disks: disks.iter().map(DiskData::from).collect(),
            disk_io: DiskIoData::from(&disk_io),
            uptime,
        };

        serde_json::to_writer(&mut out, &packet)?;
        writeln!(out)?;
        out.flush()?;

        std::thread::sleep(Duration::from_millis(interval_ms));
    }
}
