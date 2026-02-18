//! Plugin system. Supports loading dynamic shared libraries that implement
//! the `PulsePlugin` trait to add custom metric panels and data sources.
//!
//! # Safety
//! Loading dynamic libraries is inherently unsafe. Plugins must be compiled
//! against the same Rust ABI and are loaded via `libloading`.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use color_eyre::Result;

// ── Plugin trait (C-compatible interface) ────────────────────────────────────

/// Information a plugin provides about itself.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: [u8; 64],
    pub version: [u8; 16],
}

/// Metric data returned by a plugin.
#[derive(Debug, Clone)]
pub struct PluginMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
}

/// Type alias for the plugin creation function.
/// Plugins export: `extern "C" fn pulse_plugin_create() -> *mut dyn PulsePlugin`
pub type PluginCreateFn = unsafe fn() -> *mut dyn PulsePluginVTable;

/// Safe trait that plugins implement. The actual loading uses a simplified
/// vtable approach for FFI safety.
pub trait PulsePluginVTable: Send {
    /// Return plugin name.
    fn name(&self) -> &str;
    /// Return plugin version.
    fn version(&self) -> &str;
    /// Initialize the plugin.
    fn init(&mut self) -> bool;
    /// Collect metrics. Called every data refresh cycle.
    fn collect(&mut self) -> Vec<PluginMetric>;
    /// Cleanup.
    fn shutdown(&mut self);
}

// ── Plugin manager ───────────────────────────────────────────────────────────

/// Manages loaded plugins.
pub struct PluginManager {
    plugins: Vec<LoadedPlugin>,
    search_paths: Vec<PathBuf>,
}

struct LoadedPlugin {
    name: String,
    _lib: libloading::Library,
    instance: Box<dyn PulsePluginVTable>,
}

impl PluginManager {
    pub fn new() -> Self {
        let mut search_paths = Vec::new();

        // Default plugin directories
        if let Some(config_dir) = dirs::config_dir() {
            search_paths.push(config_dir.join("pulse").join("plugins"));
        }
        search_paths.push(PathBuf::from("/usr/lib/pulse/plugins"));
        search_paths.push(PathBuf::from("./plugins"));

        Self {
            plugins: Vec::new(),
            search_paths,
        }
    }

    /// Discover and load all plugins from search paths.
    pub fn discover(&mut self) {
        for search_path in self.search_paths.clone() {
            if !search_path.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&search_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "so" || e == "dylib").unwrap_or(false) {
                        if let Err(e) = self.load_plugin(&path) {
                            eprintln!("Failed to load plugin {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    /// Load a single plugin from a shared library path.
    fn load_plugin(&mut self, path: &Path) -> Result<()> {
        // SAFETY: Loading dynamic libraries is inherently unsafe.
        // We trust that plugins are compiled against the correct ABI.
        unsafe {
            let lib = libloading::Library::new(path)
                .map_err(|e| color_eyre::eyre::eyre!("Failed to load library: {}", e))?;

            let create_fn: libloading::Symbol<PluginCreateFn> = lib
                .get(b"pulse_plugin_create")
                .map_err(|e| color_eyre::eyre::eyre!("Missing symbol: {}", e))?;

            let raw = create_fn();
            if raw.is_null() {
                return Err(color_eyre::eyre::eyre!("Plugin creation returned null"));
            }

            let mut instance = Box::from_raw(raw);
            if !instance.init() {
                return Err(color_eyre::eyre::eyre!("Plugin init failed"));
            }

            let name = instance.name().to_string();
            self.plugins.push(LoadedPlugin {
                name,
                _lib: lib,
                instance,
            });
        }
        Ok(())
    }

    /// Collect metrics from all loaded plugins.
    pub fn collect_all(&mut self) -> Vec<(String, Vec<PluginMetric>)> {
        self.plugins
            .iter_mut()
            .map(|p| {
                let metrics = p.instance.collect();
                (p.name.clone(), metrics)
            })
            .collect()
    }

    /// Shutdown all plugins.
    pub fn shutdown(&mut self) {
        for plugin in &mut self.plugins {
            plugin.instance.shutdown();
        }
        self.plugins.clear();
    }

    /// Number of loaded plugins.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Names of loaded plugins.
    pub fn names(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.name.as_str()).collect()
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}
