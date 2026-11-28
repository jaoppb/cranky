//! Queries over a laid-out tree: hit-testing, finding the open popup, panel or tooltip, and
//! collecting embedded module layouts.
use super::popup::{ActivePanel, AnchoredPopup};
use super::render_node::RenderNode;
use super::styled_node::StyledNode;
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::{NodePath, SurfaceSpace};
use crate::shared::primitives::geometry::{Position, Rect};
use crate::shared::primitives::{ChildBounds, ChildModuleLayout, LayoutSurface, SizeConstraint};

const fn layout_surface_of(surface: SurfaceSpace) -> LayoutSurface {
    match surface {
        SurfaceSpace::Bar => LayoutSurface::Bar,
        SurfaceSpace::Popup => LayoutSurface::Popup,
        SurfaceSpace::Panel => LayoutSurface::Panel,
        SurfaceSpace::Tooltip => LayoutSurface::Tooltip,
    }
}

/// The size an embedded module laid out in `rect` is pinned to. The host box is the parent's
/// call (the Shadow DOM split): its resolved width — from the embedder's width, min-*, grow,
/// stretch or percentages, in whatever units the CSS used — is the child's definite width, so the
/// child fills it. A zero width is "nothing resolved yet", not a pin. Height is pinned only by an
/// explicit CSS height, so the popup keeps following its content.
fn module_constraint(rect: Rect, style: &ComputedStyle) -> SizeConstraint {
    SizeConstraint::new(
        (rect.width() > 0).then(|| rect.width()),
        style.height().is_some().then(|| rect.height()),
    )
}

impl RenderNode {
    #[must_use]
    pub(crate) fn collect_module_layouts(&self) -> Vec<ChildModuleLayout> {
        let mut layouts = Vec::new();
        self.collect_module_layouts_recursive(&mut layouts);
        layouts
    }

    fn collect_module_layouts_recursive(&self, out: &mut Vec<ChildModuleLayout>) {
        if let Some(layout) = self.module_layout() {
            out.push(layout);
        }
        for child in self.children() {
            child.collect_module_layouts_recursive(out);
        }
    }

    /// Where and how large the module this node embeds is laid out, when it embeds one.
    fn module_layout(&self) -> Option<ChildModuleLayout> {
        let Self::Module {
            rect,
            key,
            options,
            style,
            path,
            ..
        } = self
        else {
            return None;
        };
        let layout = ChildModuleLayout::new(
            key.clone(),
            ChildBounds::new(*rect, module_constraint(*rect, style)),
            layout_surface_of(path.surface()),
        );
        Some(layout.with_options(options.clone()))
    }

    #[must_use]
    pub(crate) fn hit_test(&self, pos: Position) -> Vec<&Self> {
        let mut path = Vec::new();
        self.hit_test_internal(pos, &mut path);
        path
    }

    fn hit_test_internal<'a>(&'a self, pos: Position, path: &mut Vec<&'a Self>) {
        let rect = self.rect();
        let width_i32 = i32::try_from(rect.width()).unwrap_or(i32::MAX);
        let height_i32 = i32::try_from(rect.height()).unwrap_or(i32::MAX);
        let max_x = rect.x().saturating_add(width_i32);
        let max_y = rect.y().saturating_add(height_i32);
        if pos.x() >= rect.x() && pos.x() < max_x && pos.y() >= rect.y() && pos.y() < max_y {
            path.push(self);
            if let Self::Flex { children, .. } | Self::Grid { children, .. } = self {
                for child in children {
                    child.hit_test_internal(pos, path);
                }
            }
        }
    }

    #[must_use]
    pub(crate) fn find_popup_with_anchor(&self) -> Option<AnchoredPopup<'_>> {
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
    pub(crate) fn find_panel(&self) -> Option<ActivePanel<'_>> {
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
    pub(crate) fn find_tooltip_along(&self, path: &NodePath) -> Option<&StyledNode> {
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
    use super::super::popup::{StyledPanel, StyledPopup};
    use super::*;
    use crate::features::styling::domain::ComputedStyle;
    use crate::features::vdom::domain::{
        AnchorDirection, ElementSelectors, Interactions, PanelSpec, SurfaceSpace, TextContent,
        VNode,
    };
    use crate::shared::primitives::geometry::{Position, Rect, Size};
    use crate::shared::primitives::{DynamicValue, ModuleOptions};
    use std::collections::HashMap;

    fn leaf(path: &[usize], tooltip: Option<StyledNode>) -> RenderNode {
        RenderNode::Rect {
            path: NodePath::new(SurfaceSpace::Bar, path.to_vec()),
            node_key: None,
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
            node_key: None,
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
            node_key: None,
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
        let StyledNode::Text { text, .. } = root.find_tooltip_along(&path).expect("root tooltip")
        else {
            panic!("expected StyledNode::Text");
        };
        assert_eq!(text.as_str(), "only");
    }

    #[test]
    fn test_find_tooltip_along_stops_past_the_last_child() {
        // root (tooltip "outer") -> child[0] (tooltip "child"); index 1 does not exist.
        let child = leaf(&[0], Some(tooltip_stub("child")));
        let root = flex(&[], vec![child], Some(tooltip_stub("outer")));

        let past_end = NodePath::new(SurfaceSpace::Bar, vec![1, 0]);
        assert_eq!(
            root.find_tooltip_along(&past_end).map(text_of),
            Some("outer")
        );
        let into_leaf = NodePath::new(SurfaceSpace::Bar, vec![0, 0]);
        assert_eq!(
            root.find_tooltip_along(&into_leaf).map(text_of),
            Some("child")
        );
    }

    fn text_of(node: &StyledNode) -> &str {
        let StyledNode::Text { text, .. } = node else {
            panic!("expected StyledNode::Text, got {node:?}");
        };
        text.as_str()
    }

    /// A column `x` pixels from the left, `width` wide and 20 high.
    fn column(x: i32, width: u32) -> Rect {
        Rect::new(Position::new(x, 0), Size::new(width, 20))
    }

    /// A flex node at `rect` holding `children`, with nothing open.
    fn sized_flex(rect: Rect, children: Vec<RenderNode>) -> RenderNode {
        RenderNode::Flex {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect,
            children,
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    /// A childless node at `rect` that opens `popup` and `panel` when given.
    fn opener(rect: Rect, popup: Option<StyledPopup>, panel: Option<StyledPanel>) -> RenderNode {
        RenderNode::Rect {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect,
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup,
            panel,
        }
    }

    fn popup_of(text: &str) -> StyledPopup {
        StyledPopup::new(Box::new(tooltip_stub(text)), AnchorDirection::Bottom, None)
    }

    fn panel_of(text: &str) -> StyledPanel {
        let spec = PanelSpec::new(Box::new(VNode::new_rect(
            ElementSelectors::default(),
            Interactions::default(),
        )));
        StyledPanel::new(Box::new(tooltip_stub(text)), spec.surface())
    }

    fn hit_rects(root: &RenderNode, x: i32, y: i32) -> Vec<Rect> {
        root.hit_test(Position::new(x, y))
            .into_iter()
            .map(RenderNode::rect)
            .collect()
    }

    #[test]
    fn test_hit_test_returns_every_node_down_to_the_innermost_hit() {
        let (root, left, right, inner) = (
            column(0, 100),
            column(0, 50),
            column(50, 50),
            column(60, 10),
        );
        let tree = sized_flex(
            root,
            vec![
                opener(left, None, None),
                sized_flex(right, vec![opener(inner, None, None)]),
            ],
        );

        assert_eq!(hit_rects(&tree, 65, 5), vec![root, right, inner]);
        // Each box includes its left/top edge and excludes its right/bottom one.
        assert_eq!(hit_rects(&tree, 50, 5), vec![root, right]);
        assert_eq!(hit_rects(&tree, 49, 19), vec![root, left]);
        assert_eq!(hit_rects(&tree, 100, 5), Vec::<Rect>::new());
        assert_eq!(hit_rects(&tree, 5, 20), Vec::<Rect>::new());
        assert_eq!(hit_rects(&tree, 5, -1), Vec::<Rect>::new());
    }

    /// A bar whose first child opens nothing, whose second holds one node opening the "deep"
    /// popup and panel at `column(1, 3)`, and whose third opens the "later" ones at
    /// `column(5, 7)`.
    fn bar_with_two_openers() -> RenderNode {
        let deep = opener(column(1, 3), Some(popup_of("deep")), Some(panel_of("deep")));
        let later = opener(
            column(5, 7),
            Some(popup_of("later")),
            Some(panel_of("later")),
        );
        sized_flex(
            column(0, 100),
            vec![
                opener(column(0, 1), None, None),
                sized_flex(column(0, 1), vec![deep]),
                later,
            ],
        )
    }

    /// A flex at `column(9, 9)` opening the "own" popup and panel, over a child opening "child"
    /// ones.
    fn opener_over_opener() -> RenderNode {
        let child = opener(
            column(1, 1),
            Some(popup_of("child")),
            Some(panel_of("child")),
        );
        let mut node = sized_flex(column(9, 9), vec![child]);
        let RenderNode::Flex { popup, panel, .. } = &mut node else {
            panic!("sized_flex builds a flex node");
        };
        *popup = Some(popup_of("own"));
        *panel = Some(panel_of("own"));
        node
    }

    fn anchored_popup(tree: &RenderNode) -> Option<(Rect, &str)> {
        tree.find_popup_with_anchor()
            .map(|found| (*found.anchor_rect(), text_of(found.layout())))
    }

    fn panel_text(tree: &RenderNode) -> Option<&str> {
        tree.find_panel().map(|panel| text_of(panel.layout()))
    }

    /// The first popup in depth-first order wins, a node's own before its children's, anchored
    /// to the rect of the node that opens it.
    #[test]
    fn test_find_popup_with_anchor_takes_the_first_in_depth_first_order() {
        let expected = Some((column(1, 3), "deep"));
        assert_eq!(anchored_popup(&bar_with_two_openers()), expected);
        assert_eq!(
            anchored_popup(&opener_over_opener()),
            Some((column(9, 9), "own"))
        );
        assert_eq!(anchored_popup(&sized_flex(column(0, 1), vec![])), None);
    }

    /// The first panel in depth-first order wins, a node's own before its children's.
    #[test]
    fn test_find_panel_takes_the_first_in_depth_first_order() {
        assert_eq!(panel_text(&bar_with_two_openers()), Some("deep"));
        assert_eq!(panel_text(&opener_over_opener()), Some("own"));
        assert_eq!(panel_text(&sized_flex(column(0, 1), vec![])), None);
    }

    fn module_leaf(rect: Rect, style: ComputedStyle) -> RenderNode {
        RenderNode::Module {
            path: NodePath::new(SurfaceSpace::Bar, vec![0]),
            node_key: None,
            rect,
            key: crate::shared::primitives::ModuleKey::from_name(
                crate::shared::primitives::ModuleName::new("calendar"),
            ),
            options: crate::shared::primitives::ModuleOptions::default(),
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    #[test]
    fn test_collect_module_layouts_width_is_always_the_resolved_host_width() {
        // The host box is the embedder's call: with no CSS at all its
        // width is still the child's definite width (here, its own
        // intrinsic one), and height stays free to follow content.
        let rect = Rect::new(Position::new(0, 0), Size::new(120, 40));
        let root = flex(&[], vec![module_leaf(rect, ComputedStyle::default())], None);

        let layouts = root.collect_module_layouts();
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].bounds().constraint().width(), Some(120));
        assert_eq!(layouts[0].bounds().constraint().height(), None);
        assert_eq!(layouts[0].bounds().rect(), rect);
    }

    /// An explicit CSS height pins the module's height to its slot's resolved height, not to the
    /// CSS value.
    #[test]
    fn test_collect_module_layouts_pins_an_explicit_height() {
        let rect = Rect::new(Position::new(0, 0), Size::new(300, 32));
        let mut style = ComputedStyle::default();
        style.set_height(crate::features::styling::domain::CssLength::Pixels(40.0));
        let root = flex(&[], vec![module_leaf(rect, style)], None);

        let layouts = root.collect_module_layouts();
        assert_eq!(layouts[0].bounds().constraint().height(), Some(32));
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
        assert_eq!(layouts[0].bounds().constraint().width(), Some(300));
        // height was never pinned, so it stays unconstrained even though
        // width was.
        assert_eq!(layouts[0].bounds().constraint().height(), None);
    }

    #[test]
    fn test_collect_module_layouts_min_width_override_reaches_the_child() {
        // `.calendar { min-width: 280px }` widened the host past the
        // child's own 240; the child must be laid out at the host's 280.
        let rect = Rect::new(Position::new(0, 0), Size::new(280, 40));
        let mut style = ComputedStyle::default();
        style.set_min_width(crate::features::styling::domain::CssLength::Pixels(280.0));
        let root = flex(&[], vec![module_leaf(rect, style)], None);

        let layouts = root.collect_module_layouts();
        assert_eq!(layouts[0].bounds().constraint().width(), Some(280));
    }

    #[test]
    fn test_collect_module_layouts_zero_width_host_is_not_a_pin() {
        // A host with no report yet resolves to 0; that is "nothing
        // resolved", and pinning the child to 0 would collapse it.
        let rect = Rect::new(Position::new(0, 0), Size::new(0, 0));
        let root = flex(&[], vec![module_leaf(rect, ComputedStyle::default())], None);

        let layouts = root.collect_module_layouts();
        assert_eq!(layouts[0].bounds().constraint().width(), None);
    }

    #[test]
    fn test_collect_module_layouts_carries_key_options_and_surface() {
        let rect = Rect::new(Position::new(7, 11), Size::new(90, 30));
        let hour = || DynamicValue::String("%H".to_string());
        let options = ModuleOptions::new(HashMap::from([("format".to_string(), hour())]));
        let mut node = module_leaf(rect, ComputedStyle::default());
        let RenderNode::Module {
            key,
            options: slot,
            path,
            ..
        } = &mut node
        else {
            panic!("module_leaf builds a module node");
        };
        let key = key.clone();
        *slot = options;
        *path = NodePath::new(SurfaceSpace::Popup, vec![0]);
        let root = flex(&[], vec![node], None);

        let layouts = root.collect_module_layouts();
        let [layout] = layouts.as_slice() else {
            panic!("expected one module layout, got {layouts:?}");
        };
        assert_eq!((layout.key(), layout.bounds().rect()), (&key, rect));
        assert_eq!(layout.options().get("format"), Some(&hour()));
        assert_eq!(layout.surface(), LayoutSurface::Popup);
    }
}
