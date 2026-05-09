# Cranky: Project Guidelines

- **Encapsulation:** Never expose struct fields as `pub`. Use getter methods instead.
- **Errors:** Do not use a global error module. Define errors near where they occur (locally within modules).
- **Crates:** Use `cargo add` to add new crates, then modify `Cargo.toml` to use exactly MAJOR.MINOR version (e.g., "1.0", "0.4"). Always remove the PATCH version.
- **Dead Code:** Never use `#[allow(dead_code)]`. All code must be used or removed.
- **Testing:** Maintain 80% unit test coverage.
- **Modularity:** Adhere to the addon-like structure defined in `SPEC.md`.
- **Architecture (Clean Architecture & DDD):** 
    - **Domain Layer (`src/domain`):** Contains core business logic, entities (e.g., `HyprlandState`), and application services (`CrankyApp`). It must remain independent of external frameworks.
    - **Ports Layer (`src/ports`):** Defines traits that act as gateways for the domain to interact with the outside world (e.g., `DisplayServerPort`, `WindowManagerPort`, `Canvas`).
    - **Adapters Layer (`src/adapters`):** Implements the ports using specific technologies (e.g., `WaylandAdapter`, `HyprlandAdapter`, `SkiaCanvas`).
    - **Infrastructure Layer (`src/core`):** Contains low-level technical primitives (e.g., SHM management).
    - **Module System (`src/modules`):** Implements bar features as decoupled 'mini-use-cases' that interact with the domain via signals.

## Implementation Notes

- **Positioning Logic:** Implemented left, center, and right positioning in `ModuleRegistry`. Modules must implement `measure` to provide their width. `RenderContext` provides `measure_text` for this purpose.
- **Multi-Surface Rendering Subsystem:**
    - Transitioned to a multi-surface architecture using `wl_subsurface`.
    - Each module now manages its own `ModuleSurface`, which includes a `WlSurface`, `WlSubsurface`, and its own `ShmBuffer`.
    - This allows for independent rendering and targeted input handling per module.
    - Modules render into a dedicated `PixmapMut` sized exactly to their measured dimensions.
- **Bar Refactoring:** Simplified `Bar::new` arguments by grouping them into `OutputInfo` and `WaylandGlobals` structures.
- **Bar Aesthetics:** Implemented nested `border` and `margin` configuration. Borders support size, color, and radius. Margins support top, bottom, left, and right offsets.
- **Test Infrastructure:**
    - Established 80% unit test coverage milestone.
    - Introduced `HyprlandProvider` trait to enable mocking Hyprland IO logic.
    - Scoped `mockall::automock` to `#[cfg_attr(test, ...)]` to prevent build failures when `mockall` (a dev-dependency) is missing in non-test profiles.
    - Created `test_utils.rs` with `assert_pixel_color!` and `assert_pixmap_has_color!` macros for rendering verification.
    - Achieved 100% coverage on `config.rs`, `hyprland.rs`, `workspace.rs`, `test_utils.rs`, and `utils.rs`.
    - Integrated `cargo-llvm-cov` for coverage reporting.
- **Workspace Focus Logic:** Implemented distinguishing between `active` (visible on the globally focused monitor) and `focused` (visible on a non-focused monitor) workspaces. Added `focused` boolean to `Monitor` struct and updated `WorkspaceModule` to track the globally focused monitor and render with different styling accordingly.
- **Input Handling Subsystem:**
    - Implemented a decoupled input handling system using an event queue in `CrankyState`.
    - Leverages `wl_seat` and `wl_pointer` for capturing raw Wayland events.
    - Added `Bar::find_module_by_surface` to map Wayland surfaces to their corresponding module position and index.
    - Extended `Event` enum with `PointerEnter`, `PointerLeave`, `Click`, and `Scroll` variants.
    - Introduced `ModuleRegistry::update_at` for targeted event dispatching to specific modules, avoiding global broadcasts for pointer-specific interactions.
- **Documentation:** Created comprehensive `README.md` file covering project vision, features, technical stack, installation, configuration, and architecture.

