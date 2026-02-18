//! Remote host monitoring via SSH. Spawns `ssh user@host pulse --server`
//! and reads JSON lines from the child process stdout.

#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Deserialize;

// ── Remote data types (deserialize side) ─────────────────────────────────────

/// A single packet received from a remote Pulse server.
#[derive(Deserialize, Clone, Debug)]
pub struct RemotePacket {
    pub ts: i64,
    pub cpu: RemoteCpu,
    pub mem: RemoteMem,
    pub net: RemoteNet,
    pub disks: Vec<RemoteDisk>,
    pub disk_io: RemoteDiskIo,
    pub uptime: u64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RemoteCpu {
    pub global: f64,
    pub per_core: Vec<f64>,
    pub frequencies: Vec<u64>,
    pub temperature: Option<f32>,
    pub load_avg: (f64, f64, f64),
}

#[derive(Deserialize, Clone, Debug)]
pub struct RemoteMem {
    pub used: u64,
    pub free: u64,
    pub total: u64,
    pub cached: u64,
    pub buffers: u64,
    pub swap_used: u64,
    pub swap_total: u64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RemoteNet {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_speed: f64,
    pub tx_speed: f64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RemoteDisk {
    pub name: String,
    pub mount: String,
    pub used: u64,
    pub total: u64,
    pub fs: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RemoteDiskIo {
    pub read_speed: f64,
    pub write_speed: f64,
    pub total_read: u64,
    pub total_write: u64,
    pub io_wait_pct: f64,
}

impl RemoteMem {
    pub fn usage_ratio(&self) -> f64 {
        if self.total == 0 { 0.0 } else { self.used as f64 / self.total as f64 }
    }
    pub fn swap_ratio(&self) -> f64 {
        if self.swap_total == 0 { 0.0 } else { self.swap_used as f64 / self.swap_total as f64 }
    }
}

impl RemoteDisk {
    pub fn usage_ratio(&self) -> f64 {
        if self.total == 0 { 0.0 } else { self.used as f64 / self.total as f64 }
    }
}

// ── Connection status ────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Error(String),
    Disconnected,
}

// ── Remote host state ────────────────────────────────────────────────────────

/// State for a single remote host.
#[derive(Clone, Debug)]
pub struct RemoteHost {
    /// The SSH address, e.g. "user@192.168.1.10"
    pub address: String,
    /// Latest data packet received.
    pub latest: Option<RemotePacket>,
    /// Connection status.
    pub status: ConnectionStatus,
    /// When we last received data.
    pub last_update: Option<Instant>,
}

impl RemoteHost {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
            latest: None,
            status: ConnectionStatus::Disconnected,
            last_update: None,
        }
    }

    /// Short display label: hostname or last part of address.
    pub fn label(&self) -> &str {
        self.address.split('@').last().unwrap_or(&self.address)
    }
}

// ── Remote manager ───────────────────────────────────────────────────────────

/// Manages SSH connections to remote hosts.
pub struct RemoteManager {
    /// Shared state: host address → RemoteHost data.
    pub hosts: Arc<Mutex<HashMap<String, RemoteHost>>>,
    /// SSH child processes (not shared — owned by manager).
    children: Vec<(String, Child)>,
    /// Path to the remote pulse binary.
    remote_binary: String,
    /// SSH timeout.
    timeout_secs: u64,
}

impl RemoteManager {
    pub fn new(remote_binary: &str, timeout_secs: u64) -> Self {
        Self {
            hosts: Arc::new(Mutex::new(HashMap::new())),
            children: Vec::new(),
            remote_binary: remote_binary.to_string(),
            timeout_secs,
        }
    }

    /// Start monitoring a list of hosts. Spawns SSH child processes and
    /// background reader threads.
    pub fn start(&mut self, addresses: &[String]) {
        for addr in addresses {
            // Register the host
            {
                let mut hosts = self.hosts.lock().unwrap();
                let mut host = RemoteHost::new(addr);
                host.status = ConnectionStatus::Connecting;
                hosts.insert(addr.clone(), host);
            }

            // Spawn SSH child
            let result = Command::new("ssh")
                .args([
                    "-o", "BatchMode=yes",
                    "-o", &format!("ConnectTimeout={}", self.timeout_secs),
                    "-o", "StrictHostKeyChecking=accept-new",
                    addr,
                    &self.remote_binary,
                    "--server",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();

            match result {
                Ok(mut child) => {
                    let stdout = child.stdout.take().expect("piped stdout");
                    let hosts_ref = Arc::clone(&self.hosts);
                    let addr_clone = addr.clone();

                    // Spawn reader thread
                    std::thread::spawn(move || {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            match line {
                                Ok(text) => {
                                    if let Ok(packet) = serde_json::from_str::<RemotePacket>(&text) {
                                        let mut hosts = hosts_ref.lock().unwrap();
                                        if let Some(host) = hosts.get_mut(&addr_clone) {
                                            host.latest = Some(packet);
                                            host.status = ConnectionStatus::Connected;
                                            host.last_update = Some(Instant::now());
                                        }
                                    }
                                }
                                Err(e) => {
                                    let mut hosts = hosts_ref.lock().unwrap();
                                    if let Some(host) = hosts.get_mut(&addr_clone) {
                                        host.status = ConnectionStatus::Error(e.to_string());
                                    }
                                    break;
                                }
                            }
                        }
                        // Connection ended
                        let mut hosts = hosts_ref.lock().unwrap();
                        if let Some(host) = hosts.get_mut(&addr_clone) {
                            if host.status != ConnectionStatus::Error("".into()) {
                                host.status = ConnectionStatus::Disconnected;
                            }
                        }
                    });

                    self.children.push((addr.clone(), child));
                }
                Err(e) => {
                    let mut hosts = self.hosts.lock().unwrap();
                    if let Some(host) = hosts.get_mut(addr) {
                        host.status = ConnectionStatus::Error(format!("spawn failed: {}", e));
                    }
                }
            }
        }
    }

    /// Stop all remote connections.
    pub fn stop(&mut self) {
        for (_, mut child) in self.children.drain(..) {
            let _ = child.kill();
            let _ = child.wait();
        }
        let mut hosts = self.hosts.lock().unwrap();
        for host in hosts.values_mut() {
            host.status = ConnectionStatus::Disconnected;
            host.latest = None;
        }
    }

    /// Get a snapshot of all remote host states.
    pub fn snapshot(&self) -> HashMap<String, RemoteHost> {
        self.hosts.lock().unwrap().clone()
    }
}

impl Drop for RemoteManager {
    fn drop(&mut self) {
        self.stop();
    }
}
