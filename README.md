# ⚡ Pulse — Production-Grade System Monitor

A high-performance terminal system monitor built in Rust with an animated TUI, multiple themes, GPU monitoring, anomaly detection, and a plugin system.

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

### Real-Time Monitoring
| Feature | Description |
|---------|-------------|
| **CPU** | Per-core usage, frequencies, temperature, load averages, animated sparklines |
| **Memory** | RAM & swap gauges with cache/buffers breakdown (via `/proc/meminfo`) |
| **Disk** | Per-mount usage, IO throughput (read/write speeds), IO wait percentage |
| **Network** | Aggregate + per-interface stats, TCP connection inspector |
| **GPU** | NVIDIA (via `nvidia-smi`) and AMD (via sysfs) — usage, VRAM, temp, fan, power |
| **Processes** | Sortable table with PID, CPU%, MEM, status, user, threads, nice, anomaly flags |

### Advanced Features
| Feature | Description |
|---------|-------------|
| **Anomaly Detection** | Flags processes with high CPU (>90%), high memory (>1 GB), or suspicious names |
| **Historical Metrics** | Ring-buffer engine with 5-min / 15-min / 1-hour windows at 2 Hz sampling |
| **JSON Export** | Export full metrics history to `~/pulse_metrics_<timestamp>.json` |
| **Process Control** | Send SIGTERM, SIGKILL, SIGSTOP, SIGCONT to any process |
| **Regex Filtering** | Filter process list by name with regex or substring match |
| **Plugin System** | Load dynamic `.so`/`.dylib` plugins from `~/.config/pulse/plugins/` |
| **TOML Config** | Persisted at `~/.config/pulse/config.toml` with auto-save on first run |

### TUI & Visuals
| Feature | Description |
|---------|-------------|
| **3 Themes** | Neon Cyberpunk, Monochrome Terminal, Retro Amber — cycle with `t` |
| **4 Layout Modes** | Detailed, Compact, Focus, ProcessOnly — cycle with `m` |
| **5 Views** | Overview, Network Inspector, Disk Detail, GPU, History |
| **Animations** | Pulsing header, scrolling patterns, matrix rain, gradient sparklines, glow effects |
| **60 FPS** | Configurable frame rate with adaptive refresh |

## Keyboard Controls

### Navigation
| Key | Action |
|-----|--------|
| `q` | Quit |
| `↑` / `↓` | Scroll process list |
| `PgUp` / `PgDn` | Page through processes (10 at a time) |
| `Home` / `End` | Jump to top / bottom of process list |

### Views & Modes
| Key | Action |
|-----|--------|
| `o` | Overview (default view) |
| `n` | Network inspector (per-interface stats + TCP connections) |
| `d` | Disk detail (IO throughput + disk usage table) |
| `g` | GPU monitor |
| `h` | Cycle history windows (5 min → 15 min → 1 hour) |
| `m` | Cycle layout mode (Detailed → Compact → Focus → ProcessOnly) |
| `t` | Cycle theme (Neon → Monochrome → Retro) |
| `Enter` | Toggle focus mode on current view |

### Process Management
| Key | Action |
|-----|--------|
| `s` | Cycle sort (CPU → Memory → PID → Name) |
| `f` | Open filter input (supports regex) |
| `k` | Send SIGTERM to selected process |
| `K` | Send SIGKILL to selected process |
| `z` | Suspend selected process (SIGSTOP) |
| `r` | Resume selected process (SIGCONT) |

### Other
| Key | Action |
|-----|--------|
| `e` | Export metrics history to JSON |
| `!` | Toggle security/anomaly detection mode |

## Build & Run

```bash
# Debug build
cargo run

# Release build (recommended — optimized with LTO, ~3.4 MB)
cargo build --release
./target/release/pulse
```

## Configuration

Pulse auto-creates `~/.config/pulse/config.toml` on first run:

```toml
[general]
refresh_rate_ms = 500
frame_rate = 60
default_sort = "cpu"
adaptive_refresh = true

[display]
theme = "neon"
animations = true
matrix_bg = false
layout_mode = "detailed"

[panels]
cpu = true
memory = true
disk = true
network = true
processes = true
gpu = true

[security]
enabled = false
cpu_threshold = 90.0
mem_threshold_mb = 1024
net_spike_threshold_mb = 100
```

## Project Structure

```
src/
├── main.rs              # Entry point, terminal setup, config loading
├── app.rs               # Application state, input handling, tick logic
├── config.rs            # TOML configuration system
├── event.rs             # Async event loop (keyboard + tick)
├── history.rs           # Ring-buffer metrics engine with JSON export
├── plugin.rs            # Dynamic plugin loading (libloading)
├── utils.rs             # Shared formatting helpers
├── system/
│   ├── mod.rs           # Unified SystemCollector
│   ├── cpu.rs           # Per-core CPU, frequency, temperature, load avg
│   ├── memory.rs        # RAM/swap with cache/buffers (/proc/meminfo)
│   ├── disk.rs          # Disk usage + IO throughput (/proc/diskstats)
│   ├── network.rs       # Per-interface + TCP connections (/proc/net/tcp)
│   ├── process.rs       # Extended process info, signals, anomaly detection
│   └── gpu.rs           # NVIDIA (nvidia-smi) + AMD (sysfs) GPU stats
└── ui/
    ├── mod.rs           # Render dispatcher for all layout modes
    ├── theme.rs         # 3-theme engine with gradient interpolation
    ├── animation.rs     # Pulse, scroll, matrix, glow, shimmer effects
    ├── layout.rs        # Layout mode definitions & area computation
    └── panels.rs        # All panel rendering (header, CPU, mem, net, etc.)
```

## Architecture

- **Data refresh** runs every 500 ms (configurable) via `sysinfo` + `/proc` parsing
- **UI rendering** runs at 60 FPS (configurable) for smooth animations
- **Ring buffers** store up to 1 hour of metrics at 2 Hz (7200 samples max)
- **Modular design** — system collection, state management, UI, and config are fully separated
- **Linux-optimized** — reads `/proc/meminfo`, `/proc/diskstats`, `/proc/net/tcp`, `/proc/loadavg`, sysfs thermal zones
- **Plugin-ready** — drop `.so` files into `~/.config/pulse/plugins/` with the `PulsePluginVTable` interface

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` 0.29 | Terminal UI framework |
| `crossterm` 0.28 | Cross-platform terminal I/O |
| `sysinfo` 0.32 | System statistics (CPU, memory, disk, network, processes) |
| `tokio` 1 | Async runtime for the event loop |
| `color-eyre` 0.6 | Pretty error reporting |
| `serde` + `toml` | TOML configuration serialization |
| `serde_json` | JSON metrics export |
| `nix` 0.29 | POSIX signals (SIGTERM, SIGKILL, SIGSTOP, SIGCONT) |
| `regex` | Process name filtering |
| `libloading` 0.8 | Dynamic plugin loading |
| `chrono` 0.4 | Timestamps for exports |
| `dirs` 5 | XDG config directory resolution |
