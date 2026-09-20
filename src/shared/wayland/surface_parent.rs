use crate::features::layout_engine::domain::FloatingKind;

/// Which physical Wayland surface a module's own subsurface is parented to.
///
/// Replaces the old `parent_id: Option<ModuleId>` on `SurfaceCommand`, which
/// only distinguished "has a parent" from "is the root" and always parented
/// a child to `bar.surface` regardless of where it was actually embedded
/// (gap 4). `None` on `SurfaceCommand` still means "no parent at all" — the
/// root module painting its own layer-surface buffer directly.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SurfaceParent {
    /// A child embedded in the bar's own tree — parented to `bar.surface`,
    /// exactly as every module-in-a-bar has always worked.
    Bar,
    /// A child embedded inside a popup, panel or tooltip — parented to that
    /// floating surface instead.
    Floating(FloatingKind),
}
