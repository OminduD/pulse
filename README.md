# ⚡ Pulse — Cyberpunk System Monitor

A high-performance terminal system monitor built in Rust with a neon cyberpunk aesthetic.

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)

## Features

| Feature | Description |
|---------|-------------|
| **CPU** | Real-time per-core usage with animated scrolling sparkline |
| **Memory** | Animated RAM & swap gauges with gradient colors |
| **Disk** | Per-mount usage with mini bar visualizations |
| **Network** | Upload/download speed with live sparkline graph |
| **Processes** | Sortable table with keyboard scrolling |
| **Animations** | Pulsing neon header, gradient charts, 60 FPS rendering |

## Keyboard Controls

| Key | Action |
|-----|--------|
| `q` | Quit |
| `s` | Cycle sort mode (CPU → MEM → PID → NAME) |
| `↑` / `↓` | Scroll process list |
| `PgUp` / `PgDn` | Page through processes |

## Build & Run

```bash
# Debug build
cargo run

# Release build (recommended for best performance)
cargo build --release
./target/release/pulse
```

## Project Structure

```
src/
├── main.rs     # Entry point, terminal setup, main loop
├── app.rs      # Application state, history buffers, input handling
├── system.rs   # System data collection (wraps sysinfo)
├── ui.rs       # All ratatui widget rendering
├── event.rs    # Async event loop (keyboard + tick)
└── theme.rs    # Neon cyberpunk color palette
```

## Architecture

- **Data refresh** runs every 500 ms via `sysinfo` to keep CPU overhead low
- **UI rendering** runs at ~60 FPS (16 ms tick) for smooth animations
- **Double buffering** is handled by ratatui's `Terminal` to prevent flicker
- **Modular design** separates data collection, state management, and rendering

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` | Terminal UI framework |
| `crossterm` | Cross-platform terminal I/O |
| `sysinfo` | System statistics (CPU, memory, disk, network, processes) |
| `tokio` | Async runtime for the event loop |
| `color-eyre` | Pretty error reporting |
