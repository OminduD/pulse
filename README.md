# Pulse

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)
![Platform](https://img.shields.io/badge/platform-linux-lightgrey.svg)

> **A production-grade terminal system monitor with an animated cyberpunk TUI.**

Pulse is a high-performance, asynchronous system monitoring tool written in Rust. It combines accurate system metrics with a retro-futuristic CRT aesthetic, providing a visually engaging way to track your system's health.

![Screenshot](screenshot.png)
*(Note: Please add a screenshot of the application running here)*

## ✨ Features

- **Resource Monitoring**: Real-time tracking of CPU, Memory, Disk I/O, Network, and GPU usage.
- **Process Management**: powerful process list with sorting, filtering (regex supported), and signal sending capabilities (`SIGTERM`/`SIGKILL`).
- **Cyberpunk Aesthetic**: Fully animated UI with customizable themes, CRT effects (scanlines, vignette, chromatic aberration), and fluid transitions.
- **Network Inspector**: Detailed per-interface statistics and active TCP connection tracking.
- **History & Analytics**: Ring-buffer based historical data visualization with different time windows.
- **Remote Monitoring**: Headless server mode (`--server`) to emit JSON metrics for remote visualization.
- **High Performance**: Built on `tokio` for async I/O and `ratatui` for efficient rendering.

## 🚀 Installation

### From Source

Ensure you have Rust installed (1.75+ recommended).

```bash
git clone https://github.com/OminduD/pulse.git
cd pulse
cargo install --path .
```

## 🛠️ Usage

Run the application:

```bash
pulse
```

### Server Mode

Run Pulse in headless server mode to stream metrics as JSON (useful for remote monitoring or piping to other tools):

```bash
pulse --server --interval 1000
```

## ⌨️ Keybindings

| Key | Action |
| --- | --- |
| `q` / `Ctrl+c` | Quit application |
| `s` | Cycle sort mode (CPU, Mem, PID, Name) |
| `f` | entering filter mode (type regex/text) |
| `k` | Kill selected process (`SIGTERM`) |
| `K` | Force kill selected process (`SIGKILL`) |
| `m` | Cycle layout modes |
| `t` | Cycle UI themes |
| `g` | Switch to **GPU View** |
| `n` | Switch to **Network Inspector** |
| `d` | Switch to **Disk View** |
| `h` | Switch to **History View** (cycle history windows) |
| `o` | Switch to **Overview** |
| `Enter` | Toggle Focus Mode |
| `e` | Export history to JSON |

## ⚙️ Configuration

Pulse looks for a configuration file at `~/.config/pulse/config.toml`. It will be created with default values on the first run.

Example configuration snippet:

```toml
[ui]
theme = "Cyberpunk" # or "Matrix", "Dracula", etc.
frame_rate = 60

[crt]
enabled = true
scanline_intensity = 0.3
vignette_intensity = 0.4
aberration = 0.2
glow = 0.15

[system]
refresh_rate_ms = 500
process_limit = 100
```

## 🏗️ Architecture

- **Core**: Async event loop coordinated by `tokio`, decoupling data collection from UI rendering.
- **Rendering**: Uses `ratatui` with custom widgets for charts, gauges, and the CRT post-processing effect pipeline.
- **Data**: Uses `sysinfo` for cross-platform system data and `nix` for low-level process management.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
