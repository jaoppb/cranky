use crate::features::vdom::domain::{AnchorSegment, InteractionContext, NodePath, NodeRef, VNode};

/// The three interaction refs, pre-resolved into concrete absolute
/// `NodePath`s against one specific tree (`None` if a ref's keyed anchor no
/// longer exists in that tree), so each is resolved once per tree rather
/// than once per node visited while walking it.
pub struct ResolvedInteraction {
    hovered: Option<NodePath>,
    active: Option<NodePath>,
    focused: Option<NodePath>,
    is_monitor_focused: bool,
}

impl ResolvedInteraction {
    #[must_use]
    pub fn resolve(root: &VNode, ctx: &InteractionContext) -> Self {
        Self {
            hovered: ctx.hovered().and_then(|r| resolve_target_path(root, r)),
            active: ctx.active().and_then(|r| resolve_target_path(root, r)),
            focused: ctx.focused().and_then(|r| resolve_target_path(root, r)),
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
/// or `None` if one of its keyed levels can no longer be found there.
fn resolve_target_path(root: &VNode, node_ref: &NodeRef) -> Option<NodePath> {
    let mut node = root;
    let mut path = NodePath::root();
    for segment in node_ref.anchor() {
        (node, path) = resolve_segment(node, path, segment)?;
    }
    Some(path.extend(node_ref.relative_path()))
}

/// Follows one anchor segment from `start`: every step but the last walks
/// through unkeyed nodes by position, then the keyed node is picked out of
/// that parent's children by key — its captured index is ignored, since a
/// reorder is exactly what changes it.
fn resolve_segment<'a>(
    start: &'a VNode,
    start_path: NodePath,
    segment: &AnchorSegment,
) -> Option<(&'a VNode, NodePath)> {
    let Some((_, through)) = segment.path().as_slice().split_last() else {
        return (start.key() == Some(segment.key())).then_some((start, start_path));
    };

    let mut parent = start;
    let mut parent_path = start_path;
    for &idx in through {
        parent = parent.children().get(idx)?;
        parent_path = parent_path.child(idx);
    }

    parent
        .children()
        .iter()
        .enumerate()
        .find(|(_, child)| child.key() == Some(segment.key()))
        .map(|(idx, child)| (child, parent_path.child(idx)))
}
