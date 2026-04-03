# Cranky: Refined Specifications

Cranky is a minimalist, performant, and modular bar for Hyprland, written in Rust. The primary goals are low footprint, high performance, and ease of extensibility through a modular "addon-like" architecture.

## Project Vision & Core Principles

- **Low Footprint:** Efficient use of CPU and memory. Use software rendering (`tiny-skia`) to shared memory buffers for maximum performance without the overhead of complex GPU pipelines.
- **100% Modularity:** Every visual element (except the bar frame) should be a module.
- **Reliability:** 80% unit test coverage for all core logic and module state management.
- **Transparency:** Comprehensive logging using `log` and `pretty_env_logger`.

## Tech Stack & Crates

### Core Infrastructure

- **Runtime:** `tokio` for asynchronous task management (DBus, Wayland events, timers).
- **Wayland:** `wayland-client` and `wayland-protocols` for compositor communication and `layer-shell` support.
- **Logging:** `log` with `pretty_env_logger`.
- **Error Handling:** `thiserror` for defining structured error enums.
- **Configuration:** `serde` with `toml` for parsing `~/.config/cranky/config.toml`.

### Rendering & UI

- **2D Rendering:** `tiny-skia` for drawing to shared memory buffers.
- **Text Layout:** `cosmic-text` for advanced text shaping and layout (supports complex scripts, fallback fonts, etc.).
- **Shared Memory:** `memmap2` or similar for managing Wayland shared memory buffers.

### System Integration (Addons)

- **IPC/DBus:** `zbus` (native Rust) for interacting with system services.
- **Notifications:** `zbus` (implements `org.freedesktop.Notifications`).
- **Media:** `mpris` (via `zbus`) for browser and Spotify integration.
- **Audio:** `pipewire` (via `libpipewire` or `pipewire-rs`) or `pulsectl-rs`.
- **Bluetooth:** `bluer` (official Rust BlueZ interface).
- **Network:** `zbus` (interacting with NetworkManager).
- **System Usage:** `sysinfo` for CPU, RAM, and Disk usage monitoring.

## Architecture

### 1. Core Service

- **Wayland Manager:** Handles monitor discovery, layer-shell surface creation, and buffer swapping.
- **Config Loader:** Watches for changes in `config.toml` and hot-reloads modules.
- **Module Registry:** Manages the lifecycle of active modules (init, update, render).
- **Event Bus:** An internal channel system for communication between modules and the core.

### 2. Module System ("Addon-like")

Each module must implement a `CrankyModule` trait:

- `fn init(&mut self, config: &ModuleConfig) -> Result<(), CrankyError>`
- `fn update(&mut self, event: Event) -> UpdateAction`
- `fn render(&self, pixmap: &mut Pixmap, area: Rect, context: &RenderContext)`
- `fn name(&self) -> &str`

### 3. Multi-Monitor Support

- Each monitor will have its own dedicated instance of the bar (surface).
- The `Workspace` module will filter workspaces based on the monitor's unique identifier.

## Next Steps

[ ] Gradient support to borders
[ ] Optional border color for unfocused workspace to match window
[ ] Hot-Reload for config
[ ] Get modules more modular by allowing runtime dynamic libraries (needs more analysis)
[x] Fix font family not switching
[ ] Allow the border matching the hyprland config
[ ] Change event waiting mode to allow events happening before the 100ms sleep
[x] When a new monitor is connected, the bar is appearing

## Configuration Schema (`~/.config/cranky/config.toml`)

```toml
[bar]
background = "#1a1b26"
text_color = "#c0caf5"
font_family = "JetBrainsMono Nerd Font"
font_size = 14.0
scale = 1.0
border_size = 1
border_color = "#7aa2f7"
border_radius = 8
height = 30
margin = { top = 5, bottom = 0, left = 10, right = 10 }

[[modules.left]]
name = "workspace"
enable = true

[[modules.center]]
name = "hour"
format = "%H:%M:%S"
enable = true

[[modules.right]]
name = "media"
enable = true

[[modules.right]]
name = "audio"
enable = true

[[modules.right]]
name = "network"
enable = true

[[modules.right]]
name = "system"
enable = true
show_cpu = true
show_mem = true
```
