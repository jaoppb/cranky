use super::node_key::NodeKey;
use super::node_path::NodePath;

/// Identifies a specific rendered node for the purpose of remembering
/// pointer interaction (hover/active/focus) from one render to the next.
///
/// A hit is captured as the key chain (from root) of the nearest keyed
/// ancestor of the hit node, plus the path from that ancestor down to the
/// hit node itself — rather than a single (path, key) pair — so that:
/// - matching survives an earlier sibling being removed or reordered, which
///   shifts the keyed node's own absolute path (resolved by key, not path);
/// - the keyed ancestor's unkeyed descendants (e.g. a plain text label
///   inside a keyed list item) still resolve to a concrete absolute path
///   for matching, instead of losing their identity to the ancestor's key;
/// - two different lists that happen to reuse the same key (e.g. both using
///   `"1"`) can't cross-match, because the *chain* of keys from root must
///   agree, not just the leaf key.
///
/// `anchor_keys` is empty when no ancestor (including the hit node itself)
/// declared a key — in that case `relative_path` is the node's absolute
/// path, exactly like before keys existed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct NodeRef {
    anchor_keys: Vec<NodeKey>,
    relative_path: NodePath,
}

impl NodeRef {
    #[must_use]
    pub const fn new(anchor_keys: Vec<NodeKey>, relative_path: NodePath) -> Self {
        Self {
            anchor_keys,
            relative_path,
        }
    }

    #[must_use]
    pub fn anchor_keys(&self) -> &[NodeKey] {
        &self.anchor_keys
    }

    #[must_use]
    pub const fn relative_path(&self) -> &NodePath {
        &self.relative_path
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InteractionContext {
    hovered: Option<NodeRef>,
    active: Option<NodeRef>,
    focused: Option<NodeRef>,
    is_monitor_focused: bool,
}

impl InteractionContext {
    #[must_use]
    pub const fn new(
        hovered: Option<NodeRef>,
        active: Option<NodeRef>,
        focused: Option<NodeRef>,
        is_monitor_focused: bool,
    ) -> Self {
        Self {
            hovered,
            active,
            focused,
            is_monitor_focused,
        }
    }

    #[must_use]
    pub const fn hovered(&self) -> Option<&NodeRef> {
        self.hovered.as_ref()
    }

    #[must_use]
    pub const fn active(&self) -> Option<&NodeRef> {
        self.active.as_ref()
    }

    #[must_use]
    pub const fn focused(&self) -> Option<&NodeRef> {
        self.focused.as_ref()
    }

    #[must_use]
    pub const fn is_monitor_focused(&self) -> bool {
        self.is_monitor_focused
    }
}
