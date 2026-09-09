# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Cranky is a modular Wayland bar for Hyprland. Rust 2024 edition, single binary (`src/main.rs`) + library (`src/lib.rs`).

`AGENTS.md` holds the project's coding conventions (encapsulation, `thiserror`, surgical changes, CSS class naming). Read it — this file covers commands and architecture only.

## Commands

The `Justfile` is the entry point (`just --list`). Key recipes:

| Task | Command |
| --- | --- |
| Build / run | `just build`, `just release`, `just run` |
| Tests | `just test` (= `cargo test`) |
| Single test | `cargo test <name>` — e.g. `cargo test test_find_module_clock_rhai_format` |
| Lints | `just clippy` (`--all-targets --all-features`) |
| Architecture gate | `just arch-lint` (= `cargo test --test architecture`) |
| Format | `just fmt`, `just fmt-check` |
| Coverage | `just coverage` (needs `cargo-llvm-cov` + `genhtml`) |
| Benchmarks | `just bench` (criterion, `benches/benchmarks.rs`) |
| Profiling | `just release && just record-profile` then `just analyze-profile` (see the `analyze-profile` skill in `.agents/skills/`) |
| Git hooks | `just setup-hooks` (lefthook) |

`lefthook.yml` runs clippy (with `--fix`), arch-lint, and `scripts/lint-loc.sh` pre-commit; release build + tests pre-push.

Commit messages follow Conventional Commits with a scope: `feat(popup): ...`, `refactor(wayland): ...`.

## Hard constraints (these fail the build, not just review)

- **Deny-level clippy lints** (`Cargo.toml [lints.clippy]`): `pedantic`, `nursery`, plus `unwrap_used`, `expect_used`, `indexing_slicing`, `arithmetic_side_effects`, `panic`, `as_conversions`, `string_slice`, `todo`, `unimplemented`, `unreachable`, `exit`. Use `checked_*`/`saturating_*`, `get()`, `TryFrom`, and `Result` instead. `clippy.toml` relaxes these inside `#[cfg(test)]` only.
- **`#![deny(unsafe_code)]`** in both crate roots.
- **200 production LOC per file** under `src/features/` and `src/shared/` (`#[cfg(test)]` blocks excluded). Enforced by `verify_file_loc_limits` in `tests/architecture.rs` and `scripts/lint-loc.sh`. This is why nearly every unit is a directory of small submodules — when a file grows, split it, don't allowlist it.
- **Layer dependency rules** (`arch-lint.toml`, checked by `arch_lint::check!()` in `tests/architecture.rs`): `domain` and `ports` must not import `adapters` or `app`; `features` and `shared` must not import `app`.

## Architecture

Hexagonal (ports & adapters) inside Feature-Sliced Design, driven by a reactive signal hub.

```
src/app/       composition root: registry, builtins, global state, system commands
src/features/  vertical slices, each with domain/ ports/ adapters/
src/shared/    cross-cutting infra: config, dbus, env, events, primitives,
               rendering, scripting, wayland
```

Each feature and shared unit repeats the same shape: `domain/` (pure logic and value types), `ports/` (traits), `adapters/` (concrete impls wrapping a crate — taffy, lightningcss, tiny-skia, zbus, mlua, rhai, wayland-client).

### Startup (`src/main.rs`)

Reads env → loads config → builds `SignalHub` → creates four mpsc channels (`DisplayCommand`, `LayoutEvent`, `UiCommand`, `SystemCommand`) → builds `WaylandAdapter` + `ModuleRegistry` → `CrankyApp::new` loads and spawns modules → conditionally starts secondary adapters (DBus/MPRIS/SNI/metrics/Hyprland/time) based on `app.active_signals()` → `app.run(...)`. Adapters only start if some loaded module actually subscribes to their signal.

### Signals (`src/shared/events/signals/`)

`SignalHub` owns one `tokio::sync::watch` channel per state source — config, hyprland, time, dbus, systray, metrics, mpris, module sizes, monitor scales — plus a broadcast channel for pointer events. `SignalKind` (Time, Hyprland, DBus, Systray, Metrics, Mpris) is what modules declare in their `metadata()`; a module is only woken for signals it subscribes to.

### Modules are scripts, and each one is an actor

Every visual element — including the bar itself (`bar.lua`) — is a Lua or Rhai script in `assets/widgets/`, embedded via `include_str!` in `src/app/builtins.rs` and deployed to `~/.local/share/cranky/modules/`. User overrides live in `~/.config/cranky/modules/` and take precedence. `EngineSelection::Auto` tries Lua then Rhai; `EngineSelection::Explicit` pins one.

The Rust side sees only `AnyModulePort` (`src/features/module_runtime/ports/module.rs`): `init`, `subscriptions`, `styles`, `refresh`, `render(monitor) -> VNode`, `call_function_with_args`. The Lua and Rhai adapters (`src/shared/scripting/adapters/`) both implement it — **any change to the scripting API must be mirrored in both, and both are covered by the same tests in `src/app/builtins.rs`.**

Script-side contract: define `init()`, `metadata()` (returns `subscriptions` + `styles`), `refresh()`, `render(monitor)`. Globals injected: `ui` (element constructors `flex`/`grid`/`text`/`progress`/`rect`/`image`/`module`/`popup`/`panel`, plus `ui.action.exec|call|systray`), `signals`, `config`, `sys`. `ui.module(name)` embeds a child module — that's how `bar.lua` composes the left/center/right slots.

`ModuleRegistry::spawn_all` gives each module its own `EventLoop` task (`src/features/module_runtime/adapters/event_loop/`) with its own `RenderPipeline` and its own Wayland (sub)surface, addressed by `(ModuleId, MonitorId)`.

### Render pipeline (per module, per monitor)

`src/features/module_runtime/domain/render_pipeline/`: script `render()` produces a `VNode` tree → **diff** against the previous tree (`features/vdom/adapters/differ.rs`) → **layout** via taffy (`features/layout_engine/adapters/taffy/`), with CSS resolved by lightningcss (`features/styling/`) → **paint** into a tiny-skia canvas → `SurfaceManagerPort::submit_buffer` writes into a shared-memory buffer (`shared/wayland/adapters/shm/`).

State kept per `MonitorId`, so multi-monitor is a first-class dimension throughout — module state, popups, sizes, and interactions are all keyed by monitor.

`RenderingMode` (`[render]` in config) is either `Immediate { fps_limit }` or `Timebased { duration_ms }`.

### Styling

CSS files in `assets/styles/`, deployed like widgets and resolved per module — a module declares its sheets in `metadata().styles`. Classes are scoped per module, so use `.container` / `.item` / `.root` / `.left`, never `.workspace-item`. Element tags (`bar`, `text`, `flex`, …) and pseudo-classes (`:focus`, `:hover`) are selectable.

### Hot reload

`notify` watchers on the config file, `~/.config/cranky/modules/`, and the style directories push `SystemCommand::ReloadModule` / `ReloadStyle` into `CrankyApp`'s system channel, which rebuilds the affected modules in place. Editing a widget script or CSS file requires no restart.

## Testing

Unit tests live inline in `#[cfg(test)] mod tests` next to the code; `src/test_utils.rs` holds shared macros/helpers (test-only). `tests/architecture.rs` is the only integration test and is a structural gate, not a behavior test. Target is 80%+ coverage on core logic and module state.
