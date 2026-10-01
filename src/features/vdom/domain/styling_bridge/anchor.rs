use crate::features::vdom::domain::{
    InteractionContext, NodePath, NodeRef, SurfaceSpace, VNode, resolve_segment,
};

/// The three interaction refs, pre-resolved into concrete absolute
/// `NodePath`s against one specific tree (`None` if a ref's keyed anchor no
/// longer exists in that tree), so each is resolved once per tree rather
/// than once per node visited while walking it.
#[derive(Debug)]
pub struct ResolvedInteraction {
    hovered: Option<NodePath>,
    active: Option<NodePath>,
    focused: Option<NodePath>,
    is_monitor_focused: bool,
}

impl ResolvedInteraction {
    #[must_use]
    pub fn resolve(root: &VNode, surface: SurfaceSpace, ctx: &InteractionContext) -> Self {
        Self {
            hovered: ctx
                .hovered()
                .and_then(|r| resolve_target_path(root, surface, r)),
            active: ctx
                .active()
                .and_then(|r| resolve_target_path(root, surface, r)),
            focused: ctx
                .focused()
                .and_then(|r| resolve_target_path(root, surface, r)),
            is_monitor_focused: ctx.is_monitor_focused(),
        }
    }

    #[must_use]
    pub const fn is_monitor_focused(&self) -> bool {
        self.is_monitor_focused
    }

    /// Whether `path` is the hovered node, or an ancestor of it.
    #[must_use]
    pub fn is_hovered_or_ancestor(&self, path: &NodePath) -> bool {
        self.hovered.as_ref().is_some_and(|t| t.starts_with(path))
    }

    /// Whether `path` is the active node, or an ancestor of it.
    #[must_use]
    pub fn is_active_or_ancestor(&self, path: &NodePath) -> bool {
        self.active.as_ref().is_some_and(|t| t.starts_with(path))
    }

    /// Whether `path` is exactly the focused node (focus never bubbles).
    #[must_use]
    pub fn is_focused(&self, path: &NodePath) -> bool {
        self.focused.as_ref() == Some(path)
    }
}

/// Turns a captured `NodeRef` into a concrete absolute `NodePath` in `root`,
/// or `None` if one of its keyed levels can no longer be found there, or if
/// the ref was captured in a different surface than `root` belongs to (a
/// popup's hover must never resolve against the bar's tree, and vice versa).
fn resolve_target_path(
    root: &VNode,
    surface: SurfaceSpace,
    node_ref: &NodeRef,
) -> Option<NodePath> {
    if node_ref.relative_path().surface() != surface {
        return None;
    }
    let mut node = root;
    let mut path = NodePath::root_in(surface);
    for segment in node_ref.anchor() {
        (node, path) = resolve_segment(node, path, segment)?;
    }
    Some(path.extend(node_ref.relative_path()))
}
