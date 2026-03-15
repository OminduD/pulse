//! Fan monitoring via Linux hwmon sysfs interface.

use std::path::Path;

/// Fan monitoring snapshot.
#[derive(Clone, Debug, Default)]
pub struct FanSnapshot {
    pub fans: Vec<FanInfo>,
    pub available: bool,
}

/// Single fan device data.
#[derive(Clone, Debug)]
pub struct FanInfo {
    /// Hwmon device name (e.g., "coretemp", "thinkpad", "nct6775").
    pub device_name: String,
    /// Fan label if available (e.g., "cpu_fan", "sys_fan1").
    pub label: String,
    /// Current RPM reading (None if unreadable).
    pub rpm: Option<u32>,
    /// Minimum RPM (fan*_min) if available.
    pub min_rpm: Option<u32>,
    /// Maximum RPM (fan*_max) if available.
    pub max_rpm: Option<u32>,
    /// PWM duty cycle (0-255) if controllable.
    pub pwm: Option<u8>,
    /// PWM enable mode: 0=off, 1=manual, 2=auto.
    pub pwm_mode: Option<u8>,
    /// Whether the fan file exists but couldn't be read.
    pub read_error: bool,
}

impl FanSnapshot {
    /// Collect fan data from hwmon sysfs.
    pub fn collect() -> Self {
        let hwmon_path = Path::new("/sys/class/hwmon");
        if !hwmon_path.exists() {
            return Self::default();
        }

        let mut fans = Vec::new();

        if let Ok(entries) = std::fs::read_dir(hwmon_path) {
            for entry in entries.flatten() {
                let hwmon_dir = entry.path();

                // Get device name
                let device_name = read_sysfs_string(&hwmon_dir.join("name"))
                    .unwrap_or_else(|| "unknown".to_string());

                // Scan for fan*_input files (fan1_input, fan2_input, etc.)
                if let Ok(files) = std::fs::read_dir(&hwmon_dir) {
                    for file in files.flatten() {
                        let fname = file.file_name().to_string_lossy().to_string();

                        // Match fan*_input pattern
                        if fname.starts_with("fan") && fname.ends_with("_input") {
                            let fan_num = fname
                                .trim_start_matches("fan")
                                .trim_end_matches("_input");

                            // Try to read RPM, but still add fan even if read fails
                            let (rpm, read_error) = match read_sysfs_u32(&file.path()) {
                                Some(v) => (Some(v), false),
                                None => (None, true), // File exists but couldn't read
                            };

                            // Get optional label
                            let label_path = hwmon_dir.join(format!("fan{}_label", fan_num));
                            let label = read_sysfs_string(&label_path)
                                .unwrap_or_else(|| format!("fan{}", fan_num));

                            // Get optional min/max RPM
                            let min_path = hwmon_dir.join(format!("fan{}_min", fan_num));
                            let max_path = hwmon_dir.join(format!("fan{}_max", fan_num));
                            let min_rpm = read_sysfs_u32(&min_path);
                            let max_rpm = read_sysfs_u32(&max_path);

                            // Get PWM control info if available
                            let pwm_path = hwmon_dir.join(format!("pwm{}", fan_num));
                            let pwm_enable_path =
                                hwmon_dir.join(format!("pwm{}_enable", fan_num));
                            let pwm = read_sysfs_u32(&pwm_path).map(|v| v.min(255) as u8);
                            let pwm_mode =
                                read_sysfs_u32(&pwm_enable_path).map(|v| v.min(255) as u8);

                            fans.push(FanInfo {
                                device_name: device_name.clone(),
                                label,
                                rpm,
                                min_rpm,
                                max_rpm,
                                pwm,
                                pwm_mode,
                                read_error,
                            });
                        }
                    }
                }
            }
        }

        // Sort by device name, then label for consistent ordering
        fans.sort_by(|a, b| {
            a.device_name
                .cmp(&b.device_name)
                .then(a.label.cmp(&b.label))
        });

        let available = !fans.is_empty();
        FanSnapshot { fans, available }
    }
}

impl FanInfo {
    /// Get RPM or 0 if unavailable.
    pub fn rpm_or_zero(&self) -> u32 {
        self.rpm.unwrap_or(0)
    }

    /// Calculate fan speed as percentage of max RPM (if known).
    pub fn speed_pct(&self) -> Option<f32> {
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
