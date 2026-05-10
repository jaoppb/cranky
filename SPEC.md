# Cranky: Refined Specifications

Cranky is a minimalist, performant, and modular bar for Hyprland, written in Rust. The primary goals are low footprint, high performance, and ease of extensibility through a modular "addon-like" architecture.

## Project Vision & Core Principles

- **Low Footprint:** Efficient use of CPU and memory. Use software rendering (`tiny-skia`) to shared memory buffers for maximum performance without the overhead of complex GPU pipelines.
- **100% Modularity:** Every visual element (except the bar frame) should be a module.
- **Reliability:** 80% unit test coverage for all core logic and module state management.
- **Transparency:** Comprehensive logging using `tracing`.

## Tech Stack & Crates

### Core Infrastructure

- **Runtime:** `tokio` for asynchronous task management (DBus, Wayland events, timers).
- **Wayland:** `wayland-client` and `wayland-protocols` for compositor communication and `layer-shell` support.
- **Logging:** `tracing` with `tracing-subscriber` for structured logging and diagnostics.
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

## Architecture (Reactive Hexagonal Architecture)

Cranky follows a Clean Architecture approach using the Ports & Adapters (Hexagonal) pattern, combined with DDD principles to ensure a decoupled and testable system.

### 1. Domain Layer (`src/domain`)
- **Entities & Value Objects:** Core domain models like `HyprlandState`, `Color`, and `Workspace`.
- **Application Services:** `CrankyApp` acts as the orchestrator, coordinating signals from adapters and dispatching commands.
- **SignalHub:** A reactive domain service that manages state propagation across the system.

### 2. Ports Layer (`src/ports`)
Defines the boundaries between the domain and the infrastructure. These are Rust traits that allow the domain to remain agnostic of implementation details.
- `DisplayServerPort`: Interface for Wayland/Windowing operations.
- `WindowManagerPort`: Interface for interacting with Hyprland IPC.
- `Canvas`: Interface for rendering operations.

### 3. Adapters Layer (`src/adapters`)
Implements the ports using specific frameworks and libraries.
- `WaylandAdapter`: Communicates with the Wayland compositor.
- `HyprlandAdapter`: Handles IPC communication with Hyprland.
- `SkiaCanvas`: Implements rendering using `tiny-skia`.

### 4. Infrastructure Layer (`src/core`)
Contains technical primitives that support the adapters, such as shared memory (`shm.rs`) and low-level socket handling.

### 5. Module System (`src/modules`)
Specific bar features (e.g., `WorkspaceModule`, `HourModule`) are implemented as decoupled components. They:
- Subscribe to domain signals via `SignalHub`.
- Implement the `CrankyModule` trait.
- Render their state onto a `Canvas`.

## Module System ("Addon-like")

Each module must implement a `CrankyModule` trait:

- `fn init(&mut self, config: &ModuleConfig) -> Result<(), CrankyError>`
- `fn update(&mut self, event: Event) -> UpdateAction`
- `fn measure(&self, context: &RenderContext) -> u32`
- `fn render(&self, pixmap: &mut PixmapMut, context: &RenderContext)`
- `fn name(&self) -> &str`

### 3. Multi-Monitor Support

- Each monitor will have its own dedicated instance of the bar (surface).
- The `Workspace` module will filter workspaces based on the monitor's unique identifier.

## Next Steps

[X] Gradient support to borders
[X] Optional border color for unfocused workspace to match window
[X] Hot-Reload for config
[ ] Get modules more modular by allowing runtime dynamic libraries (needs more analysis)
[x] Fix font family not switching
[ ] Allow the border matching the hyprland config
[X] Change event waiting mode to allow events happening before the 100ms sleep
[x] When a new monitor is connected, the bar is appearing
[X] Add another main loop mode that is FPS based
[X] Create Applet module
