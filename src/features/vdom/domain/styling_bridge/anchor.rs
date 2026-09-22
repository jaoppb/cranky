use crate::features::vdom::domain::{InteractionContext, NodeKey, NodePath, NodeRef, VNode};

/// The three interaction refs, pre-resolved into concrete absolute
/// `NodePath`s against one specific tree (`None` if a ref's keyed anchor no
/// longer exists in that tree). Resolving once per tree — rather than once
/// per node visited while walking it — turns an O(nodes²) anchor search
/// into O(nodes).
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
/// or `None` if its anchor (when it has one) can no longer be found there.
fn resolve_target_path(root: &VNode, node_ref: &NodeRef) -> Option<NodePath> {
    let anchor_path = if node_ref.anchor_keys().is_empty() {
        NodePath::root()
    } else {
        find_anchor(
            root,
            &NodePath::root(),
            node_ref.anchor_keys(),
            &mut Vec::new(),
        )?
    };
    Some(anchor_path.extend(node_ref.relative_path()))
}

/// Depth-first search for the node whose keyed ancestors (including itself)
/// spell out exactly `target`, in order from root. Unkeyed nodes don't
/// extend the chain, so they're transparent to the search. Since the chain
/// only ever grows on the way down, a branch whose chain-so-far isn't a
/// prefix of `target` cannot contain a match and is pruned.
fn find_anchor(
    node: &VNode,
    path: &NodePath,
    target: &[NodeKey],
    seen: &mut Vec<NodeKey>,
) -> Option<NodePath> {
    let pushed = node.key().is_some();
    if let Some(key) = node.key() {
        seen.push(key.clone());
    }

    let result = if seen.as_slice() == target {
        Some(path.clone())
    } else if target.starts_with(seen.as_slice()) {
        node.children()
            .iter()
            .enumerate()
            .find_map(|(idx, child)| find_anchor(child, &path.child(idx), target, seen))
    } else {
        None
    };

    if pushed {
        seen.pop();
    }
    result
}
