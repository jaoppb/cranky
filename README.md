# Cranky

[![CI](https://github.com/jaoppb/cranky/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/jaoppb/cranky/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/jaoppb/cranky)](https://github.com/jaoppb/cranky/releases/latest)
[![AUR](https://img.shields.io/aur/version/cranky)](https://aur.archlinux.org/packages/cranky)
[![License: MIT](https://img.shields.io/github/license/jaoppb/cranky)](LICENSE)

Cranky is a minimalist, performant, and modular bar for Hyprland, written in Rust. It is designed with a focus on low resource consumption, high performance, and ease of extensibility: every visual element, the bar included, is a small Lua or Rhai script you can override.

## 🚀 Features

- **Low Footprint:** Efficient CPU and memory usage using software rendering (`tiny-skia`) to shared memory buffers.
- **100% Modular:** Every visual element is a self-contained script module, and yours take precedence over the built-in ones.
- **Flexible Positioning:** Supports `left`, `center`, and `right` alignment for modules.
- **Styled with CSS:** Each module has its own stylesheet, scoped to it.
- **Hot Reload:** Editing the configuration, a module or a stylesheet takes effect without a restart.
- **Hyprland Integration:** Built-in workspace module with support for multi-monitor focus tracking.
- **High-Quality Typography:** Advanced text shaping and layout using `cosmic-text`.

## 🛠 Tech Stack

- **Language:** Rust, with modules in Lua (`mlua`) or Rhai
- **Runtime:** `tokio` (Asynchronous I/O)
- **Wayland:** `wayland-client` with `layer-shell` support
- **Layout and styling:** `taffy` and `lightningcss`
- **Rendering:** `tiny-skia` (2D graphics) and `cosmic-text` (Text)
- **System Integration:** `zbus` (DBus), `sysinfo`

## 📦 Installation

Cranky needs a Wayland compositor with layer-shell support; it is built for [Hyprland](https://hyprland.org/).

### Arch Linux (AUR)

- [`cranky`](https://aur.archlinux.org/packages/cranky): builds the latest release from source, and needs Rust 1.100 or newer.
- [`cranky-bin`](https://aur.archlinux.org/packages/cranky-bin): the prebuilt release binary.
- [`cranky-git`](https://aur.archlinux.org/packages/cranky-git): builds the latest `main`.

```bash
paru -S cranky   # or cranky-bin, cranky-git, with any AUR helper
```

### Nix

```bash
nix run github:jaoppb/cranky
```

The flake exposes the package as `packages.<system>.default`, for `x86_64-linux` and `aarch64-linux`.

### Release tarball

Each [release](https://github.com/jaoppb/cranky/releases) carries `cranky-vX.Y.Z-x86_64.tar.gz` with the binary, this README and the license. The binary needs glibc 2.36 or newer (Debian 12 or newer, for example) and libxkbcommon.

### From source

With [rustup](https://rustup.rs/), which installs the pinned toolchain (Rust 1.100.0) from `rust-toolchain.toml`, plus `pkg-config` and the libxkbcommon development files:

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
center = ["clock"]
right = ["metrics", "systray"]

[root.margin]
top = 8
bottom = -8
left = 8
right = 8

[render]
mode = "immediate"
fps_limit = 30

[popup]
behavior = "global"

[modules.workspace]
border_radius = 4.0
active = { background_color = "#565f89" }

[modules.clock]
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

## 🧩 Writing a module

The built-in modules (`bar`, `workspace`, `clock`, `calendar`, `metrics`, `mpris`, `systray`) are deployed to `~/.local/share/cranky/modules/`, and their stylesheets to `~/.local/share/cranky/styles/`. A file with the same name in `~/.config/cranky/modules/` or `~/.config/cranky/styles/` takes precedence, and a new name adds a module you can place in `[root]` like any other.

A module is a Lua (`.lua`) or Rhai (`.rhai`) script that defines four functions:

```lua
-- ~/.config/cranky/modules/hello.lua
local greeting = "hello"

function init()
	local options = config.module or config.options or {} -- [modules.hello]
	greeting = options.greeting or greeting
end

function metadata()
	return { subscriptions = { "time" }, styles = { "hello" } } -- styles/hello.css
end

function refresh()
	-- called when a subscribed signal changes; read it from `signals`
end

function render(monitor)
	return ui.text({ class = "label", text = greeting })
end
```

The modules in [`assets/widgets/`](assets/widgets) show the rest of the API: `ui` (`flex`, `grid`, `text`, `progress`, `rect`, `image`, `module`, `popup`, `panel` and the click actions), `signals`, `config` and `sys`. Classes are scoped to the module, so `.label` in `hello.css` styles only `hello`.

## 🧪 Development

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the setup, the gate a pull request must pass, and how issues and releases work, and [`AGENTS.md`](AGENTS.md) for the architecture and the rules.
