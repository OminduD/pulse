//! Configuration system. Reads `~/.config/pulse/config.toml` and provides
//! runtime-mutable settings with sensible defaults.

use std::fs;
use std::path::PathBuf;

use color_eyre::Result;
use serde::{Deserialize, Serialize};

// ── Default constants ────────────────────────────────────────────────────────

const DEFAULT_REFRESH_MS: u64 = 500;
const DEFAULT_FRAME_RATE: u64 = 60;

// ── Config structs ───────────────────────────────────────────────────────────

/// Top-level configuration, mirrors the TOML file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub display: DisplayConfig,
    pub panels: PanelConfig,
    pub security: SecurityConfig,
    pub remote: RemoteConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// System data refresh interval in milliseconds.
    pub refresh_rate_ms: u64,
    /// Target frame rate for UI rendering.
    pub frame_rate: u64,
    /// Default process sort mode: "cpu", "memory", "pid", "name".
    pub default_sort: String,
    /// Enable adaptive refresh (reduce rate when idle).
    pub adaptive_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    /// Active theme: "neon", "monochrome", "retro", "synthwave", "ocean".
    pub theme: String,
    /// Enable animations globally.
    pub animations: bool,
    /// Enable matrix-style background animation.
    pub matrix_bg: bool,
    /// Default layout mode: "compact", "detailed".
    pub layout_mode: String,
    /// Enable CRT post-processing effects (scanlines, vignette, aberration).
    pub crt_effects: bool,
    /// CRT scanline intensity 0.0–1.0.
    pub crt_scanline_intensity: f64,
    /// CRT vignette intensity 0.0–1.0.
    pub crt_vignette_intensity: f64,
    /// CRT chromatic aberration strength 0.0–1.0.
    pub crt_aberration: f64,
    /// CRT phosphor glow persistence 0.0–1.0.
    pub crt_glow: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelConfig {
    pub cpu: bool,
    pub memory: bool,
    pub disk: bool,
    pub network: bool,
    pub processes: bool,
    pub gpu: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoteConfig {
    /// List of remote hosts in "user@host" format.
    pub hosts: Vec<String>,
    /// Path to pulse binary on remote hosts (defaults to "pulse").
    pub remote_binary: String,
    /// SSH connection timeout in seconds.
    pub connect_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Enable security mode (highlight suspicious processes).
    pub enabled: bool,
    /// CPU threshold (%) to flag a runaway process.
    pub cpu_threshold: f32,
    /// Memory threshold (MB) to flag a memory spike.
    pub mem_threshold_mb: f64,
    /// Network spike threshold (MB/s).
    pub net_spike_threshold_mb: f64,
}

// ── Defaults ─────────────────────────────────────────────────────────────────

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            display: DisplayConfig::default(),
            panels: PanelConfig::default(),
            security: SecurityConfig::default(),
            remote: RemoteConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            refresh_rate_ms: DEFAULT_REFRESH_MS,
            frame_rate: DEFAULT_FRAME_RATE,
            default_sort: "cpu".into(),
            adaptive_refresh: true,
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            theme: "neon".into(),
            animations: true,
            matrix_bg: false,
            layout_mode: "detailed".into(),
            crt_effects: false,
            crt_scanline_intensity: 0.3,
            crt_vignette_intensity: 0.4,
            crt_aberration: 0.2,
            crt_glow: 0.15,
        }
    }
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            hosts: Vec::new(),
            remote_binary: "pulse".into(),
            connect_timeout_secs: 10,
        }
    }
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            cpu: true,
            memory: true,
            disk: true,
            network: true,
            processes: true,
            gpu: false,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cpu_threshold: 90.0,
            mem_threshold_mb: 4096.0,
            net_spike_threshold_mb: 100.0,
        }
    }
}

// ── Config I/O ───────────────────────────────────────────────────────────────

impl Config {
    /// Path to the configuration file.
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pulse")
            .join("config.toml")
    }

    /// Load config from disk, falling back to defaults if the file is missing.
    pub fn load() -> Result<Self> {
        let path = Self::path();
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let cfg: Config = toml::from_str(&contents)?;
            Ok(cfg)
        } else {
            Ok(Config::default())
        }
    }

    /// Save the current config to disk, creating directories as needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Derive frame tick interval in milliseconds from frame_rate.
    pub fn frame_tick_ms(&self) -> u64 {
        if self.general.frame_rate == 0 {
            16
        } else {
            1000 / self.general.frame_rate
        }
    }
}
