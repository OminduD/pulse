//! Application state. Owns the system collector, history engine, config,
//! plugin manager, and all UI state. Pure data — no rendering logic.

use std::collections::VecDeque;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use regex::Regex;

use crate::config::Config;
use crate::history::MetricsHistory;
use crate::remote::{RemoteHost, RemoteManager};
use crate::system::cpu::CpuSnapshot;
use crate::system::disk::{DiskInfo, DiskIoSnapshot};
use crate::system::gpu::GpuSnapshot;
use crate::system::memory::MemorySnapshot;
use crate::system::network::{InterfaceStats, NetSnapshot, TcpConnection};
use crate::system::process::{self, ProcessInfo};
use crate::system::SystemCollector;
use crate::ui::crt::CrtConfig;
use crate::ui::layout::{ActiveView, LayoutMode};
use crate::ui::theme::{Theme, ThemeId};
use std::collections::HashMap;

// ── Alert log ─────────────────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

pub struct AlertEvent {
    /// UI tick at which the alert was generated.
    pub tick: u64,
    pub severity: AlertSeverity,
    pub message: String,
}

// ── Sort mode ────────────────────────────────────────────────────────────────

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

// ── Input mode ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    FilterInput,
}

// ── App state ────────────────────────────────────────────────────────────────

pub struct App {
    collector: SystemCollector,

    // Current snapshots
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub disks: Vec<DiskInfo>,
    pub disk_io: DiskIoSnapshot,
    pub net: NetSnapshot,
    pub interfaces: Vec<InterfaceStats>,
    pub tcp_connections: Vec<TcpConnection>,
    pub processes: Vec<ProcessInfo>,
    pub gpu: GpuSnapshot,
    pub uptime: u64,

    // History engine
    pub history: MetricsHistory,

    // UI state
    pub sort_mode: SortMode,
    pub process_scroll: usize,
    pub tick_count: u64,
    pub last_data_refresh: Instant,
    pub last_tick: Instant,
    pub phase: f64,

    // Layout & view
    pub layout_mode: LayoutMode,
    pub active_view: ActiveView,

    // Theme
    pub theme: Theme,

    // Config
    pub config: Config,

    // CRT post-processing
    pub crt_enabled: bool,
    pub crt_config: CrtConfig,

    // Remote monitoring
    pub remote_manager: Option<RemoteManager>,
    pub remote_hosts: HashMap<String, RemoteHost>,
    #[allow(dead_code)]
    pub remote_selected: usize,

    // Filter
    pub filter_active: bool,
    pub filter_text: String,
    pub input_mode: InputMode,
    filter_regex: Option<Regex>,

    // Status message (displayed temporarily)
    pub status_message: Option<String>,
    status_expire: Option<Instant>,

    // System alerts log
    pub alerts: VecDeque<AlertEvent>,
    /// Throttle: PID → last tick an alert was fired for that PID.
    alert_throttle: HashMap<u32, u64>,

    // Process detail overlay (toggled with `i`)
    pub show_process_detail: bool,

    // Startup splash screen (counts down to 0)
    pub splash_remaining: u64,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mut collector = SystemCollector::new();
        let cpu = collector.cpu();
        let memory = collector.memory();
        let disks = collector.disks();
        let disk_io = DiskIoSnapshot::default();
        let net = collector.network(1.0);
        let interfaces = collector.per_interface(1.0);
        let processes = collector.processes();
        let gpu = GpuSnapshot::collect();
        let uptime = collector.uptime();

        let num_cores = collector.num_cpus();
        let history = MetricsHistory::new(num_cores);

        let theme_id = ThemeId::from_str(&config.display.theme);
        let theme = Theme::from_id(theme_id);

        let sort_mode = match config.general.default_sort.as_str() {
            "memory" | "mem" => SortMode::Memory,
            "pid" => SortMode::Pid,
            "name" => SortMode::Name,
            _ => SortMode::Cpu,
        };

        let layout_mode = match config.display.layout_mode.as_str() {
            "compact" => LayoutMode::Compact,
            _ => LayoutMode::Detailed,
        };

        // CRT config from display settings
        let crt_enabled = config.display.crt_effects;
        let crt_config = CrtConfig {
            scanline_intensity: config.display.crt_scanline_intensity,
            vignette_intensity: config.display.crt_vignette_intensity,
            aberration: config.display.crt_aberration,
            glow: config.display.crt_glow,
        };

        // Remote monitoring
        let remote_manager = if !config.remote.hosts.is_empty() {
            let mut mgr = RemoteManager::new(
                &config.remote.remote_binary,
                config.remote.connect_timeout_secs,
            );
            mgr.start(&config.remote.hosts);
            Some(mgr)
        } else {
            None
        };

        Self {
            collector,
            cpu,
            memory,
            disks,
            disk_io,
            net,
            interfaces,
            tcp_connections: Vec::new(),
            processes,
            gpu,
            uptime,
            history,
            sort_mode,
            process_scroll: 0,
            tick_count: 0,
            last_data_refresh: Instant::now(),
            last_tick: Instant::now(),
            phase: 0.0,
            layout_mode,
            active_view: ActiveView::Overview,
            theme,
            config,
            crt_enabled,
            crt_config,
            remote_manager,
            remote_hosts: HashMap::new(),
            remote_selected: 0,
            filter_active: false,
            filter_text: String::new(),
            input_mode: InputMode::Normal,
            filter_regex: None,
            status_message: None,
            status_expire: None,
            alerts: VecDeque::new(),
            alert_throttle: HashMap::new(),
            show_process_detail: false,
            splash_remaining: 180, // ~3 s at 60 fps
        }
    }

    /// Called every UI tick (~16 ms). Refreshes system data on a separate cadence.
    pub fn on_tick(&mut self) {
        self.tick_count += 1;

        // Count down splash screen
        self.splash_remaining = self.splash_remaining.saturating_sub(1);

        // Advance animation phase (full cycle every ~2 seconds at 60 FPS)
        if self.config.display.animations {
            self.phase = (self.phase + 1.0 / 90.0) % 1.0;
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        // Expire status message
        if let Some(expire) = self.status_expire {
            if now >= expire {
                self.status_message = None;
                self.status_expire = None;
            }
        }

        // Refresh heavy system data on the configured interval
        let refresh_ms = self.config.general.refresh_rate_ms;
        if now.duration_since(self.last_data_refresh).as_millis() >= refresh_ms as u128 {
            self.last_data_refresh = now;
            self.collector.refresh();

            let dt_data = dt.max(0.01);

            self.cpu = self.collector.cpu();
            self.memory = self.collector.memory();
            self.disks = self.collector.disks();
            self.disk_io = self.collector.disk_io(dt_data);
            self.net = self.collector.network(dt_data);
            self.interfaces = self.collector.per_interface(dt_data);
            self.processes = self.collector.processes();
            self.uptime = self.collector.uptime();

            // GPU refresh (less frequent due to nvidia-smi cost)
            if self.tick_count % 4 == 0 && self.config.panels.gpu {
                self.gpu = GpuSnapshot::collect();
            }

            // TCP connections (only in network view)
            if self.active_view == ActiveView::Network {
                self.tcp_connections =
                    crate::system::network::active_tcp_connections();
            }

            // Anomaly detection
            if self.config.security.enabled {
                process::detect_anomalies(
                    &mut self.processes,
                    self.config.security.cpu_threshold,
                    self.config.security.mem_threshold_mb,
                );

                // Record anomalous processes into the alerts log
                let tick = self.tick_count;
                let threshold_cpu = self.config.security.cpu_threshold;
                let threshold_mem = self.config.security.mem_threshold_mb;

                // Collect pending alerts first (separate from mutable borrows below)
                let pending: Vec<(u32, AlertSeverity, String)> = self
                    .processes
                    .iter()
                    .filter(|p| p.cpu > threshold_cpu || p.mem_mb > threshold_mem)
                    .filter_map(|p| {
                        let last = self.alert_throttle.get(&p.pid).copied().unwrap_or(0);
                        if tick.saturating_sub(last) < 600 {
                            return None;
                        }
                        let (sev, msg) = if p.cpu > threshold_cpu {
                            (
                                AlertSeverity::Warning,
                                format!("{} (PID {}) high CPU: {:.1}%", p.name, p.pid, p.cpu),
                            )
                        } else {
                            (
                                AlertSeverity::Warning,
                                format!("{} (PID {}) high MEM: {:.0} MB", p.name, p.pid, p.mem_mb),
                            )
                        };
                        Some((p.pid, sev, msg))
                    })
                    .collect();

                for (pid, sev, msg) in pending {
                    self.alert_throttle.insert(pid, tick);
                    self.push_alert(sev, msg);
                }
            }

            // Record history
            self.history.cpu_global.push(self.cpu.global);
            for (i, &usage) in self.cpu.per_core.iter().enumerate() {
                if i < self.history.cpu_per_core.len() {
                    self.history.cpu_per_core[i].push(usage);
                }
            }
            self.history.memory_ratio.push(self.memory.usage_ratio());
            self.history.swap_ratio.push(self.memory.swap_ratio());
            self.history.net_rx.push(self.net.rx_speed);
            self.history.net_tx.push(self.net.tx_speed);
            self.history.disk_read.push(self.disk_io.read_speed);
            self.history.disk_write.push(self.disk_io.write_speed);

            // Per-process CPU history for top 20 by CPU
            let mut top: Vec<_> = self.processes.iter().collect();
            top.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
            for p in top.iter().take(20) {
                self.history.record_process_cpu(p.pid, p.cpu as f64);
            }
            let active_pids: Vec<u32> = self.processes.iter().map(|p| p.pid).collect();
            self.history.prune_processes(&active_pids);

            // Sort processes
            self.sort_processes();
        }

        // Refresh remote host data (every tick is fine, it's just reading a mutex)
        if let Some(ref mgr) = self.remote_manager {
            self.remote_hosts = mgr.snapshot();
        }
    }

    /// Handle key input. Returns `true` if the app should quit.
    pub fn on_key(&mut self, key: KeyEvent) -> bool {
        // Any key dismisses the splash screen
        if self.splash_remaining > 0 {
            self.splash_remaining = 0;
            return false;
        }
        match self.input_mode {
            InputMode::FilterInput => self.handle_filter_input(key),
            InputMode::Normal => self.handle_normal_input(key),
        }
    }

    fn handle_normal_input(&mut self, key: KeyEvent) -> bool {
        // Esc always closes the process detail popup
        if key.code == KeyCode::Esc && self.show_process_detail {
            self.show_process_detail = false;
            return false;
        }

        match key.code {
            // Quit
            KeyCode::Char('q') | KeyCode::Char('Q') => return true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,

            // Sort
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.sort_mode = self.sort_mode.next();
                self.sort_processes();
                self.show_status(format!("Sort: {}", self.sort_mode.label()));
            }

            // Filter
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.input_mode = InputMode::FilterInput;
                self.filter_active = true;
                self.filter_text.clear();
                self.filter_regex = None;
            }

            // Kill process
            KeyCode::Char('k') => {
                self.kill_selected_process(false);
            }
            KeyCode::Char('K') => {
                self.kill_selected_process(true);
            }

            // Mode (layout)
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.layout_mode = self.layout_mode.next();
                self.show_status(format!("Layout: {}", self.layout_mode.label()));
            }

            // Theme
            KeyCode::Char('t') | KeyCode::Char('T') => {
                let new_id = self.theme.id.next();
                self.theme = Theme::from_id(new_id);
                self.show_status(format!("Theme: {}", new_id.label()));
            }

            // GPU view
            KeyCode::Char('g') | KeyCode::Char('G') => {
                self.active_view = ActiveView::Gpu;
                self.show_status("GPU View");
            }

            // Network view
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.active_view = ActiveView::Network;
                self.show_status("Network Inspector");
            }

            // Disk view
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.active_view = ActiveView::Disk;
                self.show_status("Disk & IO Monitor");
            }

            // History view
            KeyCode::Char('h') | KeyCode::Char('H') => {
                if self.active_view == ActiveView::History {
                    // Cycle history window
                    self.history.window = self.history.window.next();
                    self.show_status(format!("History: {}", self.history.window.label()));
                } else {
                    self.active_view = ActiveView::History;
                    self.show_status("History View");
                }
            }

            // Overview
            KeyCode::Char('o') | KeyCode::Char('O') => {
                self.active_view = ActiveView::Overview;
                self.show_status("Overview");
            }

            // Enter → focus mode
            KeyCode::Enter => {
                if self.layout_mode == LayoutMode::Focus {
                    self.layout_mode = LayoutMode::Detailed;
                } else {
                    self.layout_mode = LayoutMode::Focus;
                }
            }

            // Export history as JSON
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.export_history();
            }

            // Security mode toggle
            KeyCode::Char('!') => {
                self.config.security.enabled = !self.config.security.enabled;
                self.show_status(if self.config.security.enabled {
                    "Security Mode: ON"
                } else {
                    "Security Mode: OFF"
                });
            }

            // CRT effects toggle
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.crt_enabled = !self.crt_enabled;
                self.show_status(if self.crt_enabled {
                    "CRT Effects: ON"
                } else {
                    "CRT Effects: OFF"
                });
            }

            // Remote view
            KeyCode::Char('R') => {
                if !self.remote_hosts.is_empty() {
                    self.active_view = ActiveView::Remote;
                    self.show_status("Remote Hosts");
                } else {
                    self.show_status("No remote hosts configured");
                }
            }

            // CPU core heatmap
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.active_view = ActiveView::Heatmap;
                self.show_status("CPU Core Heatmap");
            }

            // Alerts log
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.active_view = ActiveView::Alerts;
                self.show_status("System Alerts");
            }

            // Process detail popup
            KeyCode::Char('i') | KeyCode::Char('I') => {
                self.show_process_detail = !self.show_process_detail;
            }

            // Suspend process
            KeyCode::Char('z') => {
                self.signal_selected_process(process::Signal::Stop);
            }
            // Resume process
            KeyCode::Char('r') => {
                self.signal_selected_process(process::Signal::Cont);
            }

            // Navigation
            KeyCode::Down | KeyCode::Char('j') => {
                if self.process_scroll < self.filtered_processes().len().saturating_sub(1) {
                    self.process_scroll += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('J') => {
                self.process_scroll = self.process_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                let max = self.filtered_processes().len().saturating_sub(1);
                self.process_scroll = (self.process_scroll + 20).min(max);
            }
            KeyCode::PageUp => {
                self.process_scroll = self.process_scroll.saturating_sub(20);
            }
            KeyCode::Home => {
                self.process_scroll = 0;
            }
            KeyCode::End => {
                self.process_scroll = self.filtered_processes().len().saturating_sub(1);
            }

            _ => {}
        }
        false
    }

    fn handle_filter_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.filter_active = false;
                self.filter_text.clear();
                self.filter_regex = None;
                self.process_scroll = 0;
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
                // Compile regex
                self.filter_regex = Regex::new(&self.filter_text).ok();
                if self.filter_text.is_empty() {
                    self.filter_active = false;
                }
                self.process_scroll = 0;
            }
            KeyCode::Backspace => {
                self.filter_text.pop();
            }
            KeyCode::Char(c) => {
                self.filter_text.push(c);
            }
            _ => {}
        }
        false
    }

    /// Get the filtered process list.
    pub fn filtered_processes(&self) -> Vec<&ProcessInfo> {
        if !self.filter_active || self.filter_text.is_empty() {
            return self.processes.iter().collect();
        }

        if let Some(ref re) = self.filter_regex {
            self.processes
                .iter()
                .filter(|p| re.is_match(&p.name) || re.is_match(&p.user))
                .collect()
        } else {
            // Fallback: substring match
            let lower = self.filter_text.to_lowercase();
            self.processes
                .iter()
                .filter(|p| {
                    p.name.to_lowercase().contains(&lower)
                        || p.user.to_lowercase().contains(&lower)
                })
                .collect()
        }
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

    fn kill_selected_process(&mut self, force: bool) {
        let procs = self.filtered_processes();
        if let Some(p) = procs.get(self.process_scroll) {
            let pid = p.pid;
            let name = p.name.clone();
            let signal = if force {
                process::Signal::Kill
            } else {
                process::Signal::Term
            };
            match process::send_signal(pid, signal) {
                Ok(()) => {
                    let sig_name = if force { "SIGKILL" } else { "SIGTERM" };
                    self.show_status(format!("Sent {} to {} ({})", sig_name, name, pid));
                }
                Err(e) => {
                    self.show_status(format!("Kill failed: {}", e));
                }
            }
        }
    }

    fn signal_selected_process(&mut self, signal: process::Signal) {
        let procs = self.filtered_processes();
        if let Some(p) = procs.get(self.process_scroll) {
            let pid = p.pid;
            let name = p.name.clone();
            match process::send_signal(pid, signal) {
                Ok(()) => {
                    let sig_name = match signal {
                        process::Signal::Stop => "SIGSTOP",
                        process::Signal::Cont => "SIGCONT",
                        process::Signal::Term => "SIGTERM",
                        process::Signal::Kill => "SIGKILL",
                        process::Signal::Custom(n) => {
                            // Leak is acceptable for rare status messages
                            return self.show_status(format!("Sent signal {} to {} ({})", n, name, pid));
                        }
                    };
                    self.show_status(format!("Sent {} to {} ({})", sig_name, name, pid));
                }
                Err(e) => {
                    self.show_status(format!("Signal failed: {}", e));
                }
            }
        }
    }

    fn export_history(&mut self) {
        match self.history.export_json() {
            Ok(json) => {
                let path = format!("pulse_export_{}.json", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
                match std::fs::write(&path, &json) {
                    Ok(()) => self.show_status(format!("Exported to {}", path)),
                    Err(e) => self.show_status(format!("Export error: {}", e)),
                }
            }
            Err(e) => self.show_status(format!("Export error: {}", e)),
        }
    }

    fn show_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.status_expire = Some(Instant::now() + std::time::Duration::from_secs(3));
    }

    fn push_alert(&mut self, severity: AlertSeverity, message: String) {
        const MAX_ALERTS: usize = 200;
        if self.alerts.len() >= MAX_ALERTS {
            self.alerts.pop_front();
        }
        self.alerts.push_back(AlertEvent {
            tick: self.tick_count,
            severity,
            message,
        });
    }
}
