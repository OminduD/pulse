//! Fan monitoring via Linux hwmon sysfs, laptop-specific interfaces, and MControlCenter DBus.
//! Supports: MSI EC, ThinkPad, ASUS, Dell, HP, and generic hwmon.

use std::path::Path;
use std::process::Command;

/// Fan monitoring snapshot.
#[derive(Clone, Debug, Default)]
pub struct FanSnapshot {
    pub fans: Vec<FanInfo>,
    pub available: bool,
    /// Laptop-specific fan mode (auto, silent, etc).
    pub fan_mode: Option<String>,
    /// Cooler boost / turbo status.
    pub cooler_boost: Option<bool>,
    /// Detected laptop brand.
    pub laptop_brand: Option<String>,
}

/// Single fan device data.
#[derive(Clone, Debug)]
pub struct FanInfo {
    /// Device/driver name (e.g., "thinkpad", "msi-ec", "asus-nb-wmi").
    pub device_name: String,
    /// Fan label (e.g., "CPU Fan", "GPU Fan").
    pub label: String,
    /// Current RPM reading.
    pub rpm: Option<u32>,
    /// Speed as percentage (0-100).
    pub speed_pct: Option<u8>,
    /// Minimum RPM if available.
    pub min_rpm: Option<u32>,
    /// Maximum RPM if available.
    pub max_rpm: Option<u32>,
    /// PWM duty cycle (0-255).
    pub pwm: Option<u8>,
    /// PWM mode: 0=off, 1=manual, 2=auto.
    pub pwm_mode: Option<u8>,
    /// Fan level (for ThinkPad: auto, full-speed, etc).
    pub level: Option<String>,
    /// File couldn't be read.
    pub read_error: bool,
}

impl FanSnapshot {
    /// Collect fan data from all available sources.
    pub fn collect() -> Self {
        let mut fans = Vec::new();
        let mut fan_mode = None;
        let mut cooler_boost = None;
        let mut laptop_brand = None;

        // Try MControlCenter DBus first (provides most complete MSI data)
        if let Some((mcc_fans, mode, boost)) = collect_mcontrolcenter_dbus() {
            laptop_brand = Some("MSI".to_string());
            fan_mode = mode;
            cooler_boost = boost;
            fans.extend(mcc_fans);
        }

        // Try laptop-specific interfaces if MControlCenter not available

        // 1. ThinkPad via /proc/acpi/ibm/fan
        if fans.is_empty() {
            if let Some(tp_fans) = collect_thinkpad() {
                laptop_brand = Some("ThinkPad".to_string());
                fans.extend(tp_fans);
            }
        }

        // 2. MSI EC (fallback if MControlCenter not running)
        let msi_ec_path = Path::new("/sys/devices/platform/msi-ec");
        if fans.is_empty() && msi_ec_path.exists() {
            laptop_brand = Some("MSI".to_string());
            fan_mode = read_sysfs_string(&msi_ec_path.join("fan_mode"));
            cooler_boost = read_sysfs_string(&msi_ec_path.join("cooler_boost"))
                .map(|s| s == "on");

            if let Some(f) = collect_msi_ec_fan(msi_ec_path, "cpu", "CPU Fan") {
                fans.push(f);
            }
            if let Some(f) = collect_msi_ec_fan(msi_ec_path, "gpu", "GPU Fan") {
                fans.push(f);
            }
        }

        // 3. ASUS WMI
        if fans.is_empty() {
            if let Some((asus_fans, mode)) = collect_asus() {
                laptop_brand = Some("ASUS".to_string());
                fan_mode = mode;
                fans.extend(asus_fans);
            }
        }

        // 4. Dell SMM
        if fans.is_empty() {
            if let Some(dell_fans) = collect_dell() {
                laptop_brand = Some("Dell".to_string());
                fans.extend(dell_fans);
            }
        }

        // 5. HP WMI
        if fans.is_empty() {
            if let Some(hp_fans) = collect_hp() {
                laptop_brand = Some("HP".to_string());
                fans.extend(hp_fans);
            }
        }

        // 6. Generic hwmon (fallback and additional fans)
        let hwmon_fans = collect_hwmon(&fans);
        fans.extend(hwmon_fans);

        // Sort by device name, then label
        fans.sort_by(|a, b| {
            a.device_name
                .cmp(&b.device_name)
                .then(a.label.cmp(&b.label))
        });

        let available = !fans.is_empty();
        FanSnapshot {
            fans,
            available,
            fan_mode,
            cooler_boost,
            laptop_brand,
        }
    }
}

/// Collect fan data via MControlCenter's DBus helper (if running)
fn collect_mcontrolcenter_dbus() -> Option<(Vec<FanInfo>, Option<String>, Option<bool>)> {
    // Check if mcontrolcenter.helper is available
    let check = Command::new("busctl")
        .args(["--system", "status", "mcontrolcenter.helper"])
        .output()
        .ok()?;

    if !check.status.success() {
        return None;
    }

    let mut fans = Vec::new();

    // Get CPU fan speed
    if let Some(cpu_speed) = dbus_call_int("getCPURealtimeFanSpeed") {
        // Try to get RPM from raw EC data
        let cpu_rpm = get_mcc_fan_rpm(true);

        fans.push(FanInfo {
            device_name: "msi-ec".to_string(),
            label: "CPU Fan".to_string(),
            rpm: cpu_rpm,
            speed_pct: Some(cpu_speed.min(100) as u8),
            min_rpm: None,
            max_rpm: None,
            pwm: None,
            pwm_mode: None,
            level: None,
            read_error: false,
        });
    }

    // Get GPU fan speed
    if let Some(gpu_speed) = dbus_call_int("getGPURealtimeFanSpeed") {
        let gpu_rpm = get_mcc_fan_rpm(false);

        fans.push(FanInfo {
            device_name: "msi-ec".to_string(),
            label: "GPU Fan".to_string(),
            rpm: gpu_rpm,
            speed_pct: Some(gpu_speed.min(100) as u8),
            min_rpm: None,
            max_rpm: None,
            pwm: None,
            pwm_mode: None,
            level: None,
            read_error: false,
        });
    }

    if fans.is_empty() {
        return None;
    }

    // Get fan mode
    let fan_mode = dbus_call_string("getFanMode");

    // Get cooler boost
    let cooler_boost = dbus_call_bool("getCoolerBoost");

    Some((fans, fan_mode, cooler_boost))
}

/// Call MControlCenter DBus method that returns int
fn dbus_call_int(method: &str) -> Option<i32> {
    let output = Command::new("busctl")
        .args([
            "--system",
            "call",
            "mcontrolcenter.helper",
            "/msi_ec",
            "BeardOverflow.msi_ec",
            method,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output format: "i 48"
    stdout
        .trim()
        .strip_prefix("i ")?
        .parse()
        .ok()
}

/// Call MControlCenter DBus method that returns string
fn dbus_call_string(method: &str) -> Option<String> {
    let output = Command::new("busctl")
        .args([
            "--system",
            "call",
            "mcontrolcenter.helper",
            "/msi_ec",
            "BeardOverflow.msi_ec",
            method,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output format: 's "auto"'
    stdout
        .trim()
        .strip_prefix("s \"")?
        .strip_suffix('"')
        .map(|s| s.to_string())
}

/// Call MControlCenter DBus method that returns bool
fn dbus_call_bool(method: &str) -> Option<bool> {
    let output = Command::new("busctl")
        .args([
            "--system",
            "call",
            "mcontrolcenter.helper",
            "/msi_ec",
            "BeardOverflow.msi_ec",
            method,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output format: "b true" or "b false"
    Some(stdout.trim() == "b true")
}

/// Try to get RPM from MControlCenter's raw EC data
fn get_mcc_fan_rpm(is_cpu: bool) -> Option<u32> {
    let output = Command::new("dbus-send")
        .args([
            "--system",
            "--print-reply",
            "--dest=mcontrolcenter.helper",
            "/",
            "dmitry_s93.MControlCenter.getData",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse bytes from output
    let mut bytes = Vec::new();
    for word in stdout.split_whitespace() {
        if word.len() == 2 {
            if let Ok(b) = u8::from_str_radix(word, 16) {
                bytes.push(b);
            }
        }
    }

    if bytes.len() < 256 {
        return None;
    }

    // MSI EC typical fan RPM registers (model-specific, these are common values)
    // Try multiple possible offsets
    let offsets: &[(usize, usize)] = if is_cpu {
        &[(0x44, 0x45), (0xC8, 0xC9), (0x38, 0x39)]
    } else {
        &[(0x46, 0x47), (0xCA, 0xCB), (0x3A, 0x3B)]
    };

    for &(lo, hi) in offsets {
        if hi < bytes.len() {
            let rpm_le = (bytes[lo] as u32) | ((bytes[hi] as u32) << 8);
            // Validate RPM is in reasonable range (100-10000)
            if (100..=10000).contains(&rpm_le) {
                return Some(rpm_le);
            }
        }
    }

    None
}

/// Collect ThinkPad fans from /proc/acpi/ibm/fan
fn collect_thinkpad() -> Option<Vec<FanInfo>> {
    let content = std::fs::read_to_string("/proc/acpi/ibm/fan").ok()?;
    let mut fans = Vec::new();

    let mut rpm = None;
    let mut level = None;

    for line in content.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 2 {
            let key = parts[0].trim();
            let val = parts[1].trim();

            match key {
                "speed" => rpm = val.parse().ok(),
                "level" => level = Some(val.to_string()),
                _ => {}
            }
        }
    }

    if rpm.is_some() || level.is_some() {
        fans.push(FanInfo {
            device_name: "thinkpad".to_string(),
            label: "System Fan".to_string(),
            rpm,
            speed_pct: None,
            min_rpm: None,
            max_rpm: None,
            pwm: None,
            pwm_mode: None,
            level,
            read_error: false,
        });
    }

    if fans.is_empty() {
        None
    } else {
        Some(fans)
    }
}

/// Collect MSI EC fan
fn collect_msi_ec_fan(base: &Path, subdir: &str, label: &str) -> Option<FanInfo> {
    let speed_path = base.join(subdir).join("realtime_fan_speed");
    let speed = read_sysfs_u32(&speed_path)?;

    Some(FanInfo {
        device_name: "msi-ec".to_string(),
        label: label.to_string(),
        rpm: None, // MSI EC only provides percentage
        speed_pct: Some(speed.min(100) as u8),
        min_rpm: None,
        max_rpm: None,
        pwm: None,
        pwm_mode: None,
        level: None,
        read_error: false,
    })
}

/// Collect ASUS fans from asus-nb-wmi or asus-wmi-sensors
fn collect_asus() -> Option<(Vec<FanInfo>, Option<String>)> {
    let mut fans = Vec::new();
    let mut fan_mode = None;

    // Check for asus-nb-wmi hwmon
    let hwmon_path = Path::new("/sys/class/hwmon");
    if let Ok(entries) = std::fs::read_dir(hwmon_path) {
        for entry in entries.flatten() {
            let hwmon_dir = entry.path();
            let name = read_sysfs_string(&hwmon_dir.join("name"))?;

            if name.contains("asus") {
                // Look for fan inputs
                if let Ok(files) = std::fs::read_dir(&hwmon_dir) {
                    for file in files.flatten() {
                        let fname = file.file_name().to_string_lossy().to_string();
                        if fname.starts_with("fan") && fname.ends_with("_input") {
                            let fan_num = fname
                                .trim_start_matches("fan")
                                .trim_end_matches("_input");

                            let rpm = read_sysfs_u32(&file.path());
                            let label = read_sysfs_string(
                                &hwmon_dir.join(format!("fan{}_label", fan_num)),
                            )
                            .unwrap_or_else(|| format!("Fan {}", fan_num));

                            fans.push(FanInfo {
                                device_name: "asus-wmi".to_string(),
                                label,
                                rpm,
                                speed_pct: None,
                                min_rpm: None,
                                max_rpm: None,
                                pwm: read_sysfs_u32(&hwmon_dir.join(format!("pwm{}", fan_num)))
                                    .map(|v| v.min(255) as u8),
                                pwm_mode: None,
                                level: None,
                                read_error: rpm.is_none(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Check for throttle_thermal_policy (fan mode)
    let policy_path = Path::new("/sys/devices/platform/asus-nb-wmi/throttle_thermal_policy");
    if let Some(policy) = read_sysfs_string(policy_path) {
        fan_mode = Some(match policy.as_str() {
            "0" => "Balanced".to_string(),
            "1" => "Turbo".to_string(),
            "2" => "Silent".to_string(),
            _ => policy,
        });
    }

    if fans.is_empty() {
        None
    } else {
        Some((fans, fan_mode))
    }
}

/// Collect Dell fans from dell-smm-hwmon
fn collect_dell() -> Option<Vec<FanInfo>> {
    let mut fans = Vec::new();

    let hwmon_path = Path::new("/sys/class/hwmon");
    if let Ok(entries) = std::fs::read_dir(hwmon_path) {
        for entry in entries.flatten() {
            let hwmon_dir = entry.path();
            if let Some(name) = read_sysfs_string(&hwmon_dir.join("name")) {
                if name == "dell_smm" {
                    if let Ok(files) = std::fs::read_dir(&hwmon_dir) {
                        for file in files.flatten() {
                            let fname = file.file_name().to_string_lossy().to_string();
                            if fname.starts_with("fan") && fname.ends_with("_input") {
                                let fan_num = fname
                                    .trim_start_matches("fan")
                                    .trim_end_matches("_input");

                                let rpm = read_sysfs_u32(&file.path());
                                let label = read_sysfs_string(
                                    &hwmon_dir.join(format!("fan{}_label", fan_num)),
                                )
                                .unwrap_or_else(|| format!("Fan {}", fan_num));

                                fans.push(FanInfo {
                                    device_name: "dell-smm".to_string(),
                                    label,
                                    rpm,
                                    speed_pct: None,
                                    min_rpm: None,
                                    max_rpm: None,
                                    pwm: None,
                                    pwm_mode: None,
                                    level: None,
                                    read_error: rpm.is_none(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    if fans.is_empty() {
        None
    } else {
        Some(fans)
    }
}

/// Collect HP fans
fn collect_hp() -> Option<Vec<FanInfo>> {
    let mut fans = Vec::new();

    let hwmon_path = Path::new("/sys/class/hwmon");
    if let Ok(entries) = std::fs::read_dir(hwmon_path) {
        for entry in entries.flatten() {
            let hwmon_dir = entry.path();
            if let Some(name) = read_sysfs_string(&hwmon_dir.join("name")) {
                if name.contains("hp") {
                    if let Ok(files) = std::fs::read_dir(&hwmon_dir) {
                        for file in files.flatten() {
                            let fname = file.file_name().to_string_lossy().to_string();
                            if fname.starts_with("fan") && fname.ends_with("_input") {
                                let fan_num = fname
                                    .trim_start_matches("fan")
                                    .trim_end_matches("_input");

                                let rpm = read_sysfs_u32(&file.path());
                                let label = read_sysfs_string(
                                    &hwmon_dir.join(format!("fan{}_label", fan_num)),
                                )
                                .unwrap_or_else(|| format!("Fan {}", fan_num));

                                fans.push(FanInfo {
                                    device_name: "hp-wmi".to_string(),
                                    label,
                                    rpm,
                                    speed_pct: None,
                                    min_rpm: None,
                                    max_rpm: None,
                                    pwm: None,
                                    pwm_mode: None,
                                    level: None,
                                    read_error: rpm.is_none(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    if fans.is_empty() {
        None
    } else {
        Some(fans)
    }
}

/// Collect fans from generic hwmon (excluding already-detected laptop fans)
fn collect_hwmon(existing: &[FanInfo]) -> Vec<FanInfo> {
    let mut fans = Vec::new();

    let hwmon_path = Path::new("/sys/class/hwmon");
    if !hwmon_path.exists() {
        return fans;
    }

    if let Ok(entries) = std::fs::read_dir(hwmon_path) {
        for entry in entries.flatten() {
            let hwmon_dir = entry.path();
            let device_name = read_sysfs_string(&hwmon_dir.join("name"))
                .unwrap_or_else(|| "unknown".to_string());

            // Skip if already handled by laptop-specific collector
            if device_name.contains("asus")
                || device_name == "dell_smm"
                || device_name.contains("hp")
                || device_name == "thinkpad"
            {
                continue;
            }

            // Skip amdgpu if we have MSI EC GPU fan
            if device_name == "amdgpu" && existing.iter().any(|f| f.label == "GPU Fan") {
                continue;
            }

            if let Ok(files) = std::fs::read_dir(&hwmon_dir) {
                for file in files.flatten() {
                    let fname = file.file_name().to_string_lossy().to_string();

                    if fname.starts_with("fan") && fname.ends_with("_input") {
                        let fan_num = fname
                            .trim_start_matches("fan")
                            .trim_end_matches("_input");

                        let (rpm, read_error) = match read_sysfs_u32(&file.path()) {
                            Some(v) => (Some(v), false),
                            None => (None, true),
                        };

                        let label_path = hwmon_dir.join(format!("fan{}_label", fan_num));
                        let label = read_sysfs_string(&label_path)
                            .unwrap_or_else(|| format!("fan{}", fan_num));

                        let min_rpm = read_sysfs_u32(&hwmon_dir.join(format!("fan{}_min", fan_num)));
                        let max_rpm = read_sysfs_u32(&hwmon_dir.join(format!("fan{}_max", fan_num)));
                        let pwm = read_sysfs_u32(&hwmon_dir.join(format!("pwm{}", fan_num)))
                            .map(|v| v.min(255) as u8);
                        let pwm_mode = read_sysfs_u32(&hwmon_dir.join(format!("pwm{}_enable", fan_num)))
                            .map(|v| v.min(255) as u8);

                        fans.push(FanInfo {
                            device_name: device_name.clone(),
                            label,
                            rpm,
                            speed_pct: None,
                            min_rpm,
                            max_rpm,
                            pwm,
                            pwm_mode,
                            level: None,
                            read_error,
                        });
                    }
                }
            }
        }
    }

    fans
}

impl FanInfo {
    /// Get RPM or 0 if unavailable.
    pub fn rpm_or_zero(&self) -> u32 {
        self.rpm.unwrap_or(0)
    }

    /// Get effective speed percentage.
    pub fn effective_speed_pct(&self) -> Option<f32> {
        if let Some(pct) = self.speed_pct {
            return Some(pct as f32);
        }
        // Try to calculate from RPM/max_rpm
        let rpm = self.rpm?;
        self.max_rpm.map(|max| {
            if max > 0 {
                (rpm as f32 / max as f32 * 100.0).min(100.0)
            } else {
                0.0
            }
        })
    }

    /// Get PWM percentage (0-100).
    pub fn pwm_pct(&self) -> Option<f32> {
        self.pwm.map(|pwm| pwm as f32 / 255.0 * 100.0)
    }

    /// Get human-readable PWM mode.
    pub fn pwm_mode_label(&self) -> Option<&'static str> {
        self.pwm_mode.map(|mode| match mode {
            0 => "Off",
            1 => "Manual",
            2 => "Auto",
            _ => "Unknown",
        })
    }

    /// Check if fan is running.
    pub fn is_running(&self) -> bool {
        if let Some(pct) = self.speed_pct {
            return pct > 0;
        }
        if let Some(rpm) = self.rpm {
            return rpm > 0;
        }
        false
    }
}

fn read_sysfs_string(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

fn read_sysfs_u32(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path)
        .ok()?
        .trim()
        .parse()
        .ok()
}
