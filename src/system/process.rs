//! Process data collection and management.
//! Provides extended process info, signal sending, and priority control.

use sysinfo::System;

/// Extended process information.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub name: String,
    pub cpu: f32,
    pub mem_mb: f64,
    pub mem_bytes: u64,
    pub status: String,
    pub user: String,
    pub threads: Option<u32>,
    /// Nice value (if available on Linux).
    pub nice: i32,
    /// Whether this process is flagged as anomalous.
    pub anomaly: ProcessAnomaly,
}

/// Flags for anomaly detection on individual processes.
#[derive(Clone, Debug, Default)]
pub struct ProcessAnomaly {
    pub high_cpu: bool,
    pub high_memory: bool,
    pub suspicious: bool,
}

pub fn collect(sys: &System) -> Vec<ProcessInfo> {
    sys.processes()
        .values()
        .map(|p| {
            let pid = p.pid().as_u32();
            let threads = read_thread_count(pid);
            let nice = read_nice_value(pid);
            let user = read_process_user(pid);

            ProcessInfo {
                pid,
                ppid: p.parent().map(|pp| pp.as_u32()),
                name: p.name().to_string_lossy().to_string(),
                cpu: p.cpu_usage(),
                mem_mb: p.memory() as f64 / 1_048_576.0,
                mem_bytes: p.memory(),
                status: format!("{:?}", p.status()),
                user,
                threads,
                nice,
                anomaly: ProcessAnomaly::default(),
            }
        })
        .collect()
}

/// Flag processes that exceed the given thresholds.
pub fn detect_anomalies(processes: &mut [ProcessInfo], cpu_thresh: f32, mem_thresh_mb: f64) {
    for p in processes.iter_mut() {
        p.anomaly.high_cpu = p.cpu > cpu_thresh;
        p.anomaly.high_memory = p.mem_mb > mem_thresh_mb;
        // Heuristic: suspicious if very high CPU and uncommon name
        p.anomaly.suspicious = p.cpu > cpu_thresh * 1.5;
    }
}

/// Send a signal to a process.
pub fn send_signal(pid: u32, signal: Signal) -> Result<(), String> {
    use nix::sys::signal as nix_sig;
    use nix::unistd::Pid;

    let nix_signal = match signal {
        Signal::Term => nix_sig::Signal::SIGTERM,
        Signal::Kill => nix_sig::Signal::SIGKILL,
        Signal::Stop => nix_sig::Signal::SIGSTOP,
        Signal::Cont => nix_sig::Signal::SIGCONT,
        Signal::Custom(num) => {
            nix_sig::Signal::try_from(num).map_err(|e| format!("Invalid signal: {}", e))?
        }
    };

    nix_sig::kill(Pid::from_raw(pid as i32), nix_signal)
        .map_err(|e| format!("Failed to send signal to PID {}: {}", pid, e))
}

/// Change the nice value of a process.
#[allow(dead_code)]
pub fn set_nice(pid: u32, nice: i32) -> Result<(), String> {
    // Use the renice approach via /proc
    let result = std::process::Command::new("renice")
        .args(["-n", &nice.to_string(), "-p", &pid.to_string()])
        .output();

    match result {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(String::from_utf8_lossy(&output.stderr).to_string()),
        Err(e) => Err(format!("Failed to renice: {}", e)),
    }
}

/// Supported signals for process control.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum Signal {
    Term,
    Kill,
    Stop,
    Cont,
    Custom(i32),
}

// ── /proc helpers ────────────────────────────────────────────────────────────

fn read_thread_count(pid: u32) -> Option<u32> {
    let path = format!("/proc/{}/status", pid);
    if let Ok(contents) = std::fs::read_to_string(&path) {
        for line in contents.lines() {
            if let Some(val) = line.strip_prefix("Threads:") {
                if let Ok(n) = val.trim().parse::<u32>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn read_nice_value(pid: u32) -> i32 {
    let path = format!("/proc/{}/stat", pid);
    if let Ok(contents) = std::fs::read_to_string(&path) {
        // Nice value is field 18 (0-indexed) in /proc/[pid]/stat
        // Fields are space-separated but field 1 (comm) can contain spaces in parens
        if let Some(end_paren) = contents.rfind(')') {
            let rest = &contents[end_paren + 2..]; // skip ") "
            let fields: Vec<&str> = rest.split_whitespace().collect();
            // nice is field index 16 from after comm (field 18 overall, but offset by 2)
            if fields.len() > 16 {
                return fields[16].parse().unwrap_or(0);
            }
        }
    }
    0
}

fn read_process_user(pid: u32) -> String {
    let path = format!("/proc/{}/status", pid);
    if let Ok(contents) = std::fs::read_to_string(&path) {
        for line in contents.lines() {
            if let Some(val) = line.strip_prefix("Uid:") {
                let uid_str = val.split_whitespace().next().unwrap_or("0");
                if let Ok(uid) = uid_str.parse::<u32>() {
                    return uid_to_name(uid);
                }
            }
        }
    }
    "?".to_string()
}

fn uid_to_name(uid: u32) -> String {
    // Quick lookup from /etc/passwd
    if let Ok(contents) = std::fs::read_to_string("/etc/passwd") {
        for line in contents.lines() {
            let fields: Vec<&str> = line.split(':').collect();
            if fields.len() >= 3 {
                if let Ok(entry_uid) = fields[2].parse::<u32>() {
                    if entry_uid == uid {
                        return fields[0].to_string();
                    }
                }
            }
        }
    }
    uid.to_string()
}
