/// Which tree a `ChildModuleLayout` was collected from — the bar's own
/// render tree, or one of the three floating kinds.
///
/// Kept dependency-free in `shared::primitives` rather than reusing
/// `vdom::domain::SurfaceSpace` directly, so this crate-wide vocabulary type
/// doesn't pull a `features::*` dependency into `shared::primitives`. A
/// child's `LayoutSurface` decides which physical Wayland surface its own
/// subsurface is parented to (`shared::wayland::SurfaceParent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LayoutSurface {
    #[default]
    Bar,
    Popup,
    Panel,
    Tooltip,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_bar() {
        assert_eq!(LayoutSurface::default(), LayoutSurface::Bar);
    }
}
