//! Container monitoring: Docker and Podman support via CLI.

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainerRuntime {
    Docker,
    Podman,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainerState {
    Running,
    Paused,
    Exited,
    Created,
    Restarting,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: ContainerState,
    pub runtime: ContainerRuntime,
    pub cpu_pct: Option<f64>,
    pub mem_usage_mb: Option<f64>,
    pub net_io: Option<String>,
    pub pids: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct ContainerSnapshot {
    pub containers: Vec<ContainerInfo>,
    pub available: bool,
    pub runtime: Option<ContainerRuntime>,
}

#[derive(Clone, Copy, Debug)]
pub enum ContainerAction {
    Stop,
    Restart,
    Pause,
    Unpause,
}

// JSON structs for docker ps / stats
#[derive(Deserialize)]
struct DockerPsJson {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Names")]
    names: String,
    #[serde(rename = "Image")]
    image: String,
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "State")]
    state: String,
}

#[derive(Deserialize)]
struct DockerStatsJson {
    #[serde(rename = "Container")]
    container: String,
    #[serde(rename = "CPUPerc")]
    cpu_perc: String,
    #[serde(rename = "MemUsage")]
    mem_usage: String,
    #[serde(rename = "NetIO")]
    net_io: String,
    #[serde(rename = "PIDs")]
    pids: String,
}

impl ContainerSnapshot {
    pub fn collect() -> Self {
        if let Some(containers) = collect_docker() {
            return Self {
                available: true,
                runtime: Some(ContainerRuntime::Docker),
                containers,
            };
        }
        if let Some(containers) = collect_podman() {
            return Self {
                available: true,
                runtime: Some(ContainerRuntime::Podman),
                containers,
            };
        }
        Self::default()
    }
}

pub fn container_action(
    id: &str,
    action: ContainerAction,
    runtime: &ContainerRuntime,
) -> Result<(), String> {
    let cmd = match runtime {
        ContainerRuntime::Docker => "docker",
        ContainerRuntime::Podman => "podman",
    };
    let action_str = match action {
        ContainerAction::Stop => "stop",
        ContainerAction::Restart => "restart",
        ContainerAction::Pause => "pause",
        ContainerAction::Unpause => "unpause",
    };
    let output = std::process::Command::new(cmd)
        .args([action_str, id])
        .output()
        .map_err(|e| format!("Failed to run {} {}: {}", cmd, action_str, e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// ── Docker ──────────────────────────────────────────────────────────────────

fn collect_docker() -> Option<Vec<ContainerInfo>> {
    let output = std::process::Command::new("docker")
        .args(["ps", "--all", "--format", "json"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut containers: Vec<ContainerInfo> = stdout
        .lines()
        .filter_map(|line| {
            let entry: DockerPsJson = serde_json::from_str(line).ok()?;
            Some(ContainerInfo {
                id: entry.id[..12.min(entry.id.len())].to_string(),
                name: entry.names,
                image: entry.image,
                status: entry.status,
                state: parse_state(&entry.state),
                runtime: ContainerRuntime::Docker,
                cpu_pct: None,
                mem_usage_mb: None,
                net_io: None,
                pids: None,
            })
        })
        .collect();

    if containers.is_empty() {
        return None;
    }

    // Fetch live stats for running containers
    if let Some(stats) = collect_docker_stats() {
        for stat in &stats {
            if let Some(c) = containers
                .iter_mut()
                .find(|c| stat.container.starts_with(&c.id))
            {
                c.cpu_pct = parse_percentage(&stat.cpu_perc);
                c.mem_usage_mb = parse_mem_mb(&stat.mem_usage);
                c.net_io = Some(stat.net_io.clone());
                c.pids = stat.pids.parse().ok();
            }
        }
    }

    Some(containers)
}

fn collect_docker_stats() -> Option<Vec<DockerStatsJson>> {
    let output = std::process::Command::new("docker")
        .args(["stats", "--no-stream", "--format", "json"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(
        stdout
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect(),
    )
}

// ── Podman ──────────────────────────────────────────────────────────────────

fn collect_podman() -> Option<Vec<ContainerInfo>> {
    let output = std::process::Command::new("podman")
        .args(["ps", "--all", "--format", "json"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Podman outputs a JSON array
    let entries: Vec<serde_json::Value> = serde_json::from_str(&stdout).ok()?;

    let containers: Vec<ContainerInfo> = entries
        .iter()
        .filter_map(|v| {
            let id = v.get("Id")?.as_str()?.to_string();
            let names = v
                .get("Names")
                .and_then(|n| n.as_array())
                .and_then(|arr| arr.first())
                .and_then(|n| n.as_str())
                .unwrap_or("?")
                .to_string();
            let image = v.get("Image")?.as_str()?.to_string();
            let status = v
                .get("Status")
                .and_then(|s| s.as_str())
                .unwrap_or("?")
                .to_string();
            let state = v
                .get("State")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown");

            Some(ContainerInfo {
                id: id[..12.min(id.len())].to_string(),
                name: names,
                image,
                status,
                state: parse_state(state),
                runtime: ContainerRuntime::Podman,
                cpu_pct: None,
                mem_usage_mb: None,
                net_io: None,
                pids: None,
            })
        })
        .collect();

    if containers.is_empty() {
        None
    } else {
        Some(containers)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn parse_state(s: &str) -> ContainerState {
    match s.to_lowercase().as_str() {
        "running" => ContainerState::Running,
        "paused" => ContainerState::Paused,
        "exited" => ContainerState::Exited,
        "created" => ContainerState::Created,
        "restarting" => ContainerState::Restarting,
        _ => ContainerState::Unknown,
    }
}

fn parse_percentage(s: &str) -> Option<f64> {
    s.trim_end_matches('%').trim().parse().ok()
}

fn parse_mem_mb(s: &str) -> Option<f64> {
    let usage_part = s.split('/').next()?.trim();
    if let Some(val) = usage_part.strip_suffix("GiB") {
        return val.trim().parse::<f64>().ok().map(|v| v * 1024.0);
    }
    if let Some(val) = usage_part.strip_suffix("MiB") {
        return val.trim().parse().ok();
    }
    if let Some(val) = usage_part.strip_suffix("KiB") {
        return val.trim().parse::<f64>().ok().map(|v| v / 1024.0);
    }
    None
}
