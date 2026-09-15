use super::popup::{ActivePanel, AnchoredPopup};
use super::render_node::RenderNode;
use super::styled_node::StyledNode;
use crate::features::vdom::domain::NodePath;
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::{ChildModuleLayout, SizeConstraint};

impl RenderNode {
    #[must_use]
    pub fn collect_module_layouts(&self) -> Vec<ChildModuleLayout> {
        let mut layouts = Vec::new();
        self.collect_module_layouts_recursive(&mut layouts);
        layouts
    }

    fn collect_module_layouts_recursive(&self, out: &mut Vec<ChildModuleLayout>) {
        match self {
            Self::Flex { children, .. } | Self::Grid { children, .. } => {
                for child in children {
                    child.collect_module_layouts_recursive(out);
                }
            }
            Self::Module { rect, key, style, .. } => {
                // A pin is "the parent set an explicit width/height" — taffy
                // already resolved it to this slot's rect, whatever units the
                // CSS was in, so the resolved pixels are the constraint value.
                // min-*/max-* clamp the slot but never pin it, so they don't
                // produce a constraint here.
                let constraint = SizeConstraint::new(
                    style.width().is_some().then(|| rect.width()),
                    style.height().is_some().then(|| rect.height()),
                );
                out.push(ChildModuleLayout::new(key.clone(), *rect, constraint));
            }
            _ => {}
        }
    }

    #[must_use]
    pub fn hit_test(&self, pos: Position) -> Vec<&Self> {
        let mut path = Vec::new();
        self.hit_test_internal(pos, &mut path);
        path
    }

    fn hit_test_internal<'a>(&'a self, pos: Position, path: &mut Vec<&'a Self>) {
        let r = self.rect();
        let width_i32 = i32::try_from(r.width()).unwrap_or(i32::MAX);
        let height_i32 = i32::try_from(r.height()).unwrap_or(i32::MAX);
        let max_x = r.x().saturating_add(width_i32);
        let max_y = r.y().saturating_add(height_i32);
        if pos.x() >= r.x() && pos.x() < max_x && pos.y() >= r.y() && pos.y() < max_y {
            path.push(self);
            if let Self::Flex { children, .. } | Self::Grid { children, .. } = self {
                for child in children {
                    child.hit_test_internal(pos, path);
                }
            }
        }
    }

    #[must_use]
    pub fn find_popup_with_anchor(&self) -> Option<AnchoredPopup<'_>> {
        if let Some(popup) = self.popup() {
            return Some(AnchoredPopup::new(self.rect(), popup));
        }
        if let Self::Flex { children, .. } | Self::Grid { children, .. } = self {
            for child in children {
                if let Some(found) = child.find_popup_with_anchor() {
                    return Some(found);
                }
            }
        }
        None
    }

    #[must_use]
    pub fn find_panel(&self) -> Option<ActivePanel<'_>> {
        if let Some(panel) = self.panel() {
            return Some(ActivePanel::new(panel));
        }
        if let Self::Flex { children, .. } | Self::Grid { children, .. } = self {
            for child in children {
                if let Some(found) = child.find_panel() {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Finds the tooltip belonging to the innermost node along `path` that
    /// carries one — the same "deepest ancestor wins" rule
    /// `PointerHandler` used when it walked a hit-test result directly,
    /// reproduced here as a path walk since the pipeline doesn't hit-test.
    ///
    /// `path`'s indices are used to descend; its surface is the caller's
    /// responsibility — this must already be the tree that surface names.
    #[must_use]
    pub fn find_tooltip_along(&self, path: &NodePath) -> Option<&StyledNode> {
        let mut current = self;
        let mut found = current.tooltip();
        for &index in path.as_slice() {
            let (Self::Flex { children, .. } | Self::Grid { children, .. }) = current else {
                break;
            };
            let Some(child) = children.get(index) else {
                break;
            };
            current = child;
            if let Some(tooltip) = current.tooltip() {
                found = Some(tooltip);
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::ComputedStyle;
    use crate::features::vdom::domain::{SurfaceSpace, TextContent};
    use crate::shared::primitives::geometry::{Position, Rect, Size};

    fn leaf(path: &[usize], tooltip: Option<StyledNode>) -> RenderNode {
        RenderNode::Rect {
            path: NodePath::new(SurfaceSpace::Bar, path.to_vec()),
            rect: Rect::new(Position::new(0, 0), Size::new(0, 0)),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: tooltip.map(Box::new),
            popup: None,
            panel: None,
        }
    }

    fn tooltip_stub(text: &str) -> StyledNode {
        StyledNode::Text {
            path: NodePath::root_in(SurfaceSpace::Tooltip),
            text: TextContent::new(text.to_string()),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    fn flex(path: &[usize], children: Vec<RenderNode>, tooltip: Option<StyledNode>) -> RenderNode {
        RenderNode::Flex {
            path: NodePath::new(SurfaceSpace::Bar, path.to_vec()),
            rect: Rect::new(Position::new(0, 0), Size::new(0, 0)),
            children,
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: tooltip.map(Box::new),
            popup: None,
            panel: None,
        }
    }

    #[test]
    fn test_find_tooltip_along_prefers_deepest() {
        // root (tooltip "outer") -> child[0] (no tooltip) -> child[0][0] (tooltip "inner")
        let inner = leaf(&[0, 0], Some(tooltip_stub("inner")));
        let middle = flex(&[0], vec![inner], None);
        let root = flex(&[], vec![middle], Some(tooltip_stub("outer")));

        let path = NodePath::new(SurfaceSpace::Bar, vec![0, 0]);
        let found = root.find_tooltip_along(&path);
        let StyledNode::Text { text, .. } = found.expect("expected a tooltip") else {
            panic!("expected StyledNode::Text");
        };
        assert_eq!(text.as_str(), "inner");
    }

    #[test]
    fn test_find_tooltip_along_falls_back_to_ancestor() {
        // root (tooltip "outer") -> child[0] (no tooltip, no further tooltip below)
        let middle = flex(&[0], vec![leaf(&[0, 0], None)], None);
        let root = flex(&[], vec![middle], Some(tooltip_stub("outer")));

        let path = NodePath::new(SurfaceSpace::Bar, vec![0, 0]);
        let found = root.find_tooltip_along(&path);
        let StyledNode::Text { text, .. } = found.expect("expected the ancestor's tooltip") else {
            panic!("expected StyledNode::Text");
        };
        assert_eq!(text.as_str(), "outer");
    }

    #[test]
    fn test_find_tooltip_along_none_when_nothing_on_path_has_one() {
        let middle = flex(&[0], vec![leaf(&[0, 0], None)], None);
        let root = flex(&[], vec![middle], None);

        let path = NodePath::new(SurfaceSpace::Bar, vec![0, 0]);
        assert!(root.find_tooltip_along(&path).is_none());
    }

    #[test]
    fn test_find_tooltip_along_stops_at_out_of_bounds_index() {
        // Path descends past a leaf (no children) — must not panic, and
        // must still return whatever was found up to that point.
        let root = leaf(&[], Some(tooltip_stub("only")));
        let path = NodePath::new(SurfaceSpace::Bar, vec![5, 2]);
        let StyledNode::Text { text, .. } = root.find_tooltip_along(&path).expect("root tooltip") else {
            panic!("expected StyledNode::Text");
        };
        assert_eq!(text.as_str(), "only");
    }

    fn module_leaf(rect: Rect, style: ComputedStyle) -> RenderNode {
        RenderNode::Module {
            path: NodePath::new(SurfaceSpace::Bar, vec![0]),
            rect,
            key: crate::shared::primitives::ModuleKey::from_name(
                crate::shared::primitives::ModuleName::new("calendar"),
            ),
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    #[test]
    fn test_collect_module_layouts_no_pin_yields_no_constraint() {
        let rect = Rect::new(Position::new(0, 0), Size::new(120, 40));
        let root = flex(&[], vec![module_leaf(rect, ComputedStyle::default())], None);

        let layouts = root.collect_module_layouts();
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].constraint().width(), None);
        assert_eq!(layouts[0].constraint().height(), None);
        assert_eq!(*layouts[0].bounds(), rect);
    }

    #[test]
    fn test_collect_module_layouts_pin_reads_resolved_pixels() {
        // Whatever units the CSS was in (px, %, calc()), taffy already
        // resolved the pin to this slot's rect — the constraint is read
        // from the resolved pixels, not re-derived from the CSS value.
        let rect = Rect::new(Position::new(0, 0), Size::new(300, 40));
        let mut style = ComputedStyle::default();
        style.set_width(crate::features::styling::domain::CssLength::Percent(100.0));
        let root = flex(&[], vec![module_leaf(rect, style)], None);

        let layouts = root.collect_module_layouts();
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].constraint().width(), Some(300));
        // height was never pinned, so it stays unconstrained even though
        // width was.
        assert_eq!(layouts[0].constraint().height(), None);
    }

    #[test]
    fn test_collect_module_layouts_min_max_do_not_pin() {
        // min-*/max-* clamp the parent's slot but never force an exact
        // value, so they must not produce a constraint for the child.
        let rect = Rect::new(Position::new(0, 0), Size::new(280, 40));
        let mut style = ComputedStyle::default();
        style.set_min_width(crate::features::styling::domain::CssLength::Px(280.0));
        let root = flex(&[], vec![module_leaf(rect, style)], None);

        let layouts = root.collect_module_layouts();
        assert_eq!(layouts[0].constraint().width(), None);
    }
}
