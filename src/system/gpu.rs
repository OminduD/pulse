//! GPU monitoring: NVIDIA (nvidia-smi) and AMD (sysfs) support.

/// GPU information snapshot.
#[derive(Clone, Debug, Default)]
pub struct GpuSnapshot {
    pub gpus: Vec<GpuInfo>,
    pub available: bool,
}

/// Single GPU device data.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: GpuVendor,
    /// GPU utilization 0–100%.
    pub usage_pct: f32,
    /// Memory used in MiB.
    pub mem_used_mib: u64,
    /// Memory total in MiB.
    pub mem_total_mib: u64,
    /// Temperature in °C.
    pub temperature: Option<f32>,
    /// Fan speed percentage.
    pub fan_pct: Option<f32>,
    /// Power draw in watts.
    pub power_watts: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Unknown,
}

impl GpuSnapshot {
    /// Collect GPU data from available sources.
    pub fn collect() -> Self {
        let mut gpus = Vec::new();

        // Try NVIDIA first
        if let Some(nvidia_gpus) = collect_nvidia() {
            gpus.extend(nvidia_gpus);
        }

        // Try AMD via sysfs
        if let Some(amd_gpus) = collect_amd() {
            gpus.extend(amd_gpus);
        }

        let available = !gpus.is_empty();
        GpuSnapshot { gpus, available }
    }
}

/// Collect NVIDIA GPU data via nvidia-smi CLI.
fn collect_nvidia() -> Option<Vec<GpuInfo>> {
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu,fan.speed,power.draw",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let gpus: Vec<GpuInfo> = stdout
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if fields.len() < 4 {
                return None;
            }

            Some(GpuInfo {
                name: fields[0].to_string(),
                vendor: GpuVendor::Nvidia,
                usage_pct: fields[1].parse().unwrap_or(0.0),
                mem_used_mib: fields[2].parse().unwrap_or(0),
                mem_total_mib: fields[3].parse().unwrap_or(0),
                temperature: fields.get(4).and_then(|s| s.parse().ok()),
                fan_pct: fields.get(5).and_then(|s| s.parse().ok()),
                power_watts: fields.get(6).and_then(|s| s.parse().ok()),
            })
        })
        .collect();

    if gpus.is_empty() {
        None
    } else {
        Some(gpus)
    }
}

/// Collect AMD GPU data from sysfs (`/sys/class/drm/card*/device/`).
fn collect_amd() -> Option<Vec<GpuInfo>> {
    let drm_path = std::path::Path::new("/sys/class/drm");
    if !drm_path.exists() {
        return None;
    }

    let mut gpus = Vec::new();

    if let Ok(entries) = std::fs::read_dir(drm_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("card") || name.contains('-') {
                continue;
            }

            let device_path = entry.path().join("device");

            // Check if it's an AMD GPU by looking for amdgpu-specific files
            let gpu_busy_path = device_path.join("gpu_busy_percent");
            if !gpu_busy_path.exists() {
                continue;
            }

            let usage_pct = read_sysfs_f32(&gpu_busy_path).unwrap_or(0.0);
            let temperature = read_sysfs_f32(&device_path.join("hwmon").join("hwmon0").join("temp1_input"))
                .map(|t| t / 1000.0); // millidegrees → degrees

            let mem_used = read_sysfs_u64(&device_path.join("mem_info_vram_used")).unwrap_or(0) / (1024 * 1024);
            let mem_total = read_sysfs_u64(&device_path.join("mem_info_vram_total")).unwrap_or(0) / (1024 * 1024);

            gpus.push(GpuInfo {
                name: format!("AMD GPU ({})", name),
                vendor: GpuVendor::Amd,
                usage_pct,
                mem_used_mib: mem_used,
                mem_total_mib: mem_total,
                temperature,
                fan_pct: None,
                power_watts: None,
            });
        }
    }

    if gpus.is_empty() {
        None
    } else {
        Some(gpus)
    }
}

fn read_sysfs_f32(path: &std::path::Path) -> Option<f32> {
    std::fs::read_to_string(path)
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn read_sysfs_u64(path: &std::path::Path) -> Option<u64> {
    std::fs::read_to_string(path)
        .ok()?
        .trim()
        .parse()
        .ok()
}
