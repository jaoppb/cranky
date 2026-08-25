# Cranky

Cranky is a minimalist, performant, and modular bar for Hyprland, written in Rust. It is designed with a focus on low resource consumption, high performance, and ease of extensibility through a modular "addon-like" architecture.

## 🚀 Features

- **Low Footprint:** Efficient CPU and memory usage using software rendering (`tiny-skia`) to shared memory buffers.
- **100% Modular:** Every visual element is a self-contained module managed by a dynamic registry.
- **Flexible Positioning:** Supports `left`, `center`, and `right` alignment for modules.
- **Rich Aesthetics:** Customizable borders (size, color, radius) and margins (top, bottom, left, right).
- **Hyprland Integration:** Built-in workspace module with support for multi-monitor focus tracking.
- **High-Quality Typography:** Advanced text shaping and layout using `cosmic-text`.
- **Reliable:** Targeted 80%+ unit test coverage for core logic and module states.

## 🛠 Tech Stack

- **Language:** Rust
- **Runtime:** `tokio` (Asynchronous I/O)
- **Wayland:** `wayland-client` with `layer-shell` support
- **Rendering:** `tiny-skia` (2D graphics) and `cosmic-text` (Text)
- **System Integration:** `zbus` (DBus), `sysinfo`

## 📦 Installation

### Prerequisites

- Rust toolchain (latest stable)
- A Wayland compositor (optimized for [Hyprland](https://hyprland.org/))
- Development libraries for Wayland and potentially DBus (depending on your distro)

### Build

```bash
git clone https://github.com/jaoppb/cranky.git
cd cranky
cargo build --release
```

The binary will be available at `target/release/cranky`.

## ⚙️ Configuration

Cranky looks for its configuration file at `~/.config/cranky/config.toml`.

```toml
[root]
name = "bar"
height = 40
left = ["workspace"]
center = ["hour"]
right = ["metrics", "systray"]

[root.margin]
top = 8
bottom = -8
left = 8
right = 8

[render]
mode = "immediate"
fps_limit = 30

[modules.workspace]
border_radius = 4.0
active = { background_color = "#565f89" }

[modules.hour]
format = "%H:%M:%S %d/%m/%Y"

[modules.metrics]
network = false
temperature = false

[modules.systray]
show_icons = true
show_titles = false
icon_size = 22

[modules.mpris]
show_icon = true
```

## 🏗 Architecture

Cranky is split into two main components:

1. **Core Service:** Handles monitor discovery, Wayland surface management, configuration hot-reloading, and the module registry.
2. **Module System:** An "addon-like" trait-based system where each module (`workspace`, `systray`, `hour`, etc.) implements a standard lifecycle: `init`, `update`, `measure`, and `render`.

## 🧪 Development

### Guidelines

- **Encapsulation:** Struct fields are private; use getter methods.
- **Error Handling:** Errors are defined locally within modules using `thiserror`.
- **Testing:** New features must include unit tests. Use `cargo-llvm-cov` for coverage reports.

### Running Tests

```bash
cargo test
# For coverage reporting:
cargo llvm-cov
```
