//! Application state. Owns the system collector and all history buffers.
//! Pure data — no rendering logic lives here.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};

use crate::system::{
    CpuSnapshot, DiskInfo, MemorySnapshot, NetSnapshot, ProcessInfo, SystemCollector,
};

// ── History buffer size (seconds of data at 2 Hz refresh) ────────────────────
const HISTORY_LEN: usize = 120;

/// How the process table is sorted.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Cpu,
    Memory,
    Pid,
    Name,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            Self::Cpu => Self::Memory,
            Self::Memory => Self::Pid,
            Self::Pid => Self::Name,
            Self::Name => Self::Cpu,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU%",
            Self::Memory => "MEM",
            Self::Pid => "PID",
            Self::Name => "NAME",
        }
    }
}

// ── App state ────────────────────────────────────────────────────────────────

pub struct App {
    collector: SystemCollector,

    // Current snapshots
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub disks: Vec<DiskInfo>,
    pub net: NetSnapshot,
    pub processes: Vec<ProcessInfo>,

    // History rings for charts (global CPU & per-core)
    pub cpu_history: Vec<f64>,
    pub net_rx_history: Vec<f64>,
    pub net_tx_history: Vec<f64>,

    // UI state
    pub sort_mode: SortMode,
    pub process_scroll: usize,
    pub tick_count: u64,
    pub last_data_refresh: Instant,
    pub last_tick: Instant,

    // Animation phase (0.0–1.0 cycling)
    pub phase: f64,
}

impl App {
    pub fn new() -> Self {
        let mut collector = SystemCollector::new();
        let cpu = collector.cpu();
        let memory = collector.memory();
        let disks = collector.disks();
        let net = collector.network(1.0);
        let processes = collector.processes();

        Self {
            collector,
            cpu,
            memory,
            disks,
            net,
            processes,
            cpu_history: vec![0.0; HISTORY_LEN],
            net_rx_history: vec![0.0; HISTORY_LEN],
            net_tx_history: vec![0.0; HISTORY_LEN],
            sort_mode: SortMode::Cpu,
            process_scroll: 0,
            tick_count: 0,
            last_data_refresh: Instant::now(),
            last_tick: Instant::now(),
            phase: 0.0,
        }
    }

    /// Called every UI tick (~16 ms). Refreshes system data every 500 ms.
    pub fn on_tick(&mut self) {
        self.tick_count += 1;

        // Advance animation phase (full cycle every 2 seconds at 60 FPS)
        self.phase = (self.phase + 1.0 / 120.0) % 1.0;

        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        // Only refresh heavy sysinfo data every ~500 ms
        if now.duration_since(self.last_data_refresh).as_millis() >= 500 {
            self.last_data_refresh = now;
            self.collector.refresh();

            self.cpu = self.collector.cpu();
            self.memory = self.collector.memory();
            self.disks = self.collector.disks();
            self.net = self.collector.network(dt.max(0.01));
            self.processes = self.collector.processes();

            // Push to history rings
            push_ring(&mut self.cpu_history, self.cpu.global, HISTORY_LEN);
            push_ring(&mut self.net_rx_history, self.net.rx_speed, HISTORY_LEN);
            push_ring(&mut self.net_tx_history, self.net.tx_speed, HISTORY_LEN);

            // Sort processes
            self.sort_processes();
        }
    }

    /// Handle key input. Returns `true` if the app should quit.
    pub fn on_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => return true,
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.sort_mode = self.sort_mode.next();
                self.sort_processes();
            }
            KeyCode::Down => {
                if self.process_scroll < self.processes.len().saturating_sub(1) {
                    self.process_scroll += 1;
                }
            }
            KeyCode::Up => {
                self.process_scroll = self.process_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.process_scroll = (self.process_scroll + 20).min(
                    self.processes.len().saturating_sub(1),
                );
            }
            KeyCode::PageUp => {
                self.process_scroll = self.process_scroll.saturating_sub(20);
            }
            _ => {}
        }
        false
    }

    fn sort_processes(&mut self) {
        match self.sort_mode {
            SortMode::Cpu => self
                .processes
                .sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal)),
            SortMode::Memory => self.processes.sort_by(|a, b| {
                b.mem_mb
                    .partial_cmp(&a.mem_mb)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortMode::Pid => self.processes.sort_by_key(|p| p.pid),
            SortMode::Name => self.processes.sort_by(|a, b| a.name.cmp(&b.name)),
        }
    }
}

/// Push a value into a ring buffer, keeping fixed capacity.
fn push_ring(buf: &mut Vec<f64>, val: f64, cap: usize) {
    buf.push(val);
    if buf.len() > cap {
        buf.remove(0);
    }
}
