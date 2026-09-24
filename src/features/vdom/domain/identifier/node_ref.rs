use super::node_key::NodeKey;
use super::node_path::NodePath;

/// One keyed level of a `NodeRef`'s anchor: the path from the previous
/// keyed level (or the root) down to this keyed node, plus its key.
///
/// Every step of `path` except the last goes through unkeyed nodes and is
/// followed positionally. The last step is only where the keyed node sat
/// when captured — it's resolved by `key` among that parent's children
/// instead, which is what lets a keyed list reorder without losing the
/// interaction. An empty `path` means this level is the starting node
/// itself (only possible for a keyed root).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnchorSegment {
    path: NodePath,
    key: NodeKey,
}

impl AnchorSegment {
    #[must_use]
    pub const fn new(path: NodePath, key: NodeKey) -> Self {
        Self { path, key }
    }

    #[must_use]
    pub const fn path(&self) -> &NodePath {
        &self.path
    }

    #[must_use]
    pub const fn key(&self) -> &NodeKey {
        &self.key
    }
}

/// Identifies a specific rendered node for the purpose of remembering
/// pointer interaction (hover/active/focus) from one render to the next.
///
/// A hit is captured as an anchor — one `AnchorSegment` per keyed ancestor
/// of the hit node, from root down to the nearest one — plus the path from
/// that nearest keyed ancestor down to the hit node itself, so that:
/// - matching survives a keyed list reordering or losing an earlier
///   sibling, which shifts the keyed node's absolute path (each keyed level
///   is resolved by key, not position);
/// - the keyed ancestor's unkeyed descendants (e.g. a plain text label
///   inside a keyed list item) still resolve to a concrete absolute path;
/// - two lists reusing the same key (e.g. both using `"1"`) can't
///   cross-match, even when their containers are unkeyed, because the
///   unkeyed levels between keys are part of the anchor positionally.
///
/// Keys only protect the levels they're on: an unkeyed sibling appearing or
/// disappearing above a keyed list shifts the positional part and drops the
/// interaction for that render, exactly as it would for fully unkeyed
/// content. `anchor` is empty when no ancestor (including the hit node
/// itself) declared a key — `relative_path` is then the absolute path.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct NodeRef {
    anchor: Vec<AnchorSegment>,
    relative_path: NodePath,
}

impl NodeRef {
    #[must_use]
    pub const fn new(anchor: Vec<AnchorSegment>, relative_path: NodePath) -> Self {
        Self {
            anchor,
            relative_path,
        }
    }

    #[must_use]
    pub fn anchor(&self) -> &[AnchorSegment] {
        &self.anchor
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
