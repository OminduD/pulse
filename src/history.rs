//! Historical metrics engine. Uses fixed-capacity ring buffers to store
//! time-series data for CPU, memory, network, and disk metrics.
//! Supports multiple time windows (5m, 15m, 1h) and JSON export.

use std::collections::VecDeque;

use serde::Serialize;

// ── Ring buffer ──────────────────────────────────────────────────────────────

/// A fixed-capacity ring buffer backed by `VecDeque` for O(1) push/pop.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RingBuffer {
    buf: VecDeque<f64>,
    capacity: usize,
}

#[allow(dead_code)]
impl RingBuffer {
    #[allow(dead_code)]
    pub fn new(capacity: usize) -> Self {
        let mut buf = VecDeque::with_capacity(capacity);
        buf.resize(capacity, 0.0);
        Self { buf, capacity }
    }

    /// Push a new value, dropping the oldest if at capacity.
    pub fn push(&mut self, value: f64) {
        if self.buf.len() >= self.capacity {
            self.buf.pop_front();
        }
        self.buf.push_back(value);
    }

    /// Get all values as a slice-like iterator (oldest first).
    pub fn as_slice(&self) -> Vec<f64> {
        self.buf.iter().copied().collect()
    }

    /// Get the last N values (most recent).
    pub fn last_n(&self, n: usize) -> Vec<f64> {
        let skip = self.buf.len().saturating_sub(n);
        self.buf.iter().skip(skip).copied().collect()
    }

    /// Get the most recent value.
    pub fn latest(&self) -> f64 {
        self.buf.back().copied().unwrap_or(0.0)
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear all values to zero.
    pub fn clear(&mut self) {
        self.buf.clear();
        self.buf.resize(self.capacity, 0.0);
    }
}

// ── Time window ──────────────────────────────────────────────────────────────

/// Supported history time windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryWindow {
    FiveMin,
    FifteenMin,
    OneHour,
}

impl HistoryWindow {
    pub fn label(self) -> &'static str {
        match self {
            Self::FiveMin => "5m",
            Self::FifteenMin => "15m",
            Self::OneHour => "1h",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::FiveMin => Self::FifteenMin,
            Self::FifteenMin => Self::OneHour,
            Self::OneHour => Self::FiveMin,
        }
    }

    /// Number of data points for this window at 2 Hz (one sample every 500ms).
    pub fn sample_count(self) -> usize {
        match self {
            Self::FiveMin => 600,    // 5 * 60 * 2
            Self::FifteenMin => 1800, // 15 * 60 * 2
            Self::OneHour => 7200,   // 60 * 60 * 2
        }
    }
}

// ── Metrics history store ────────────────────────────────────────────────────

/// Stores all historical metrics in ring buffers sized for the maximum
/// time window (1 hour at 2 Hz = 7200 samples).
#[derive(Debug, Clone)]
pub struct MetricsHistory {
    /// Global CPU usage history.
    pub cpu_global: RingBuffer,
    /// Per-core CPU usage histories (one ring per core).
    pub cpu_per_core: Vec<RingBuffer>,
    /// Memory usage ratio (0.0–1.0).
    pub memory_ratio: RingBuffer,
    /// Swap usage ratio.
    pub swap_ratio: RingBuffer,
    /// Network RX speed (bytes/sec).
    pub net_rx: RingBuffer,
    /// Network TX speed (bytes/sec).
    pub net_tx: RingBuffer,
    /// Disk read speed (bytes/sec).
    pub disk_read: RingBuffer,
    /// Disk write speed (bytes/sec).
    pub disk_write: RingBuffer,
    /// Current view window.
    pub window: HistoryWindow,
    /// Per-process CPU history (PID → ring buffer), limited to top N.
    pub process_cpu: std::collections::HashMap<u32, RingBuffer>,
}

const MAX_HISTORY: usize = 7200; // 1 hour at 2 Hz
const PROCESS_HISTORY_LEN: usize = 120; // 1 minute of sparkline per process

impl MetricsHistory {
    pub fn new(num_cores: usize) -> Self {
        Self {
            cpu_global: RingBuffer::new(MAX_HISTORY),
            cpu_per_core: (0..num_cores)
                .map(|_| RingBuffer::new(MAX_HISTORY))
                .collect(),
            memory_ratio: RingBuffer::new(MAX_HISTORY),
            swap_ratio: RingBuffer::new(MAX_HISTORY),
            net_rx: RingBuffer::new(MAX_HISTORY),
            net_tx: RingBuffer::new(MAX_HISTORY),
            disk_read: RingBuffer::new(MAX_HISTORY),
            disk_write: RingBuffer::new(MAX_HISTORY),
            window: HistoryWindow::FiveMin,
            process_cpu: std::collections::HashMap::new(),
        }
    }

    /// Record per-process CPU usage for top processes.
    pub fn record_process_cpu(&mut self, pid: u32, cpu: f64) {
        let ring = self
            .process_cpu
            .entry(pid)
            .or_insert_with(|| RingBuffer::new(PROCESS_HISTORY_LEN));
        ring.push(cpu);
    }

    /// Prune process histories for PIDs that no longer exist.
    pub fn prune_processes(&mut self, active_pids: &[u32]) {
        self.process_cpu.retain(|pid, _| active_pids.contains(pid));
    }

    /// Get data points for the current time window from a ring buffer.
    pub fn windowed_data(&self, ring: &RingBuffer) -> Vec<f64> {
        ring.last_n(self.window.sample_count())
    }

    /// Export all current metrics as JSON.
    pub fn export_json(&self) -> color_eyre::Result<String> {
        let snapshot = HistoryExport {
            timestamp: chrono::Utc::now().to_rfc3339(),
            window: self.window.label().to_string(),
            cpu_global: self.windowed_data(&self.cpu_global),
            memory_ratio: self.windowed_data(&self.memory_ratio),
            net_rx: self.windowed_data(&self.net_rx),
            net_tx: self.windowed_data(&self.net_tx),
            disk_read: self.windowed_data(&self.disk_read),
            disk_write: self.windowed_data(&self.disk_write),
        };
        Ok(serde_json::to_string_pretty(&snapshot)?)
    }
}

/// JSON export schema.
#[derive(Serialize)]
struct HistoryExport {
    timestamp: String,
    window: String,
    cpu_global: Vec<f64>,
    memory_ratio: Vec<f64>,
    net_rx: Vec<f64>,
    net_tx: Vec<f64>,
    disk_read: Vec<f64>,
    disk_write: Vec<f64>,
}
