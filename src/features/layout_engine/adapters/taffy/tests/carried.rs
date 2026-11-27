//! What every kind of node carries over unchanged to the node it is laid out as: its key, its
//! click and hover actions, and the tooltip, popup and panel it opens.
use super::super::TaffyLayoutAdapter;
use super::support::{lay_out, root_path, text_leaf};
use crate::features::layout_engine::domain::{
    RenderNode, StyledNode, StyledPanel, StyledPopup, TextContent,
};
use crate::features::styling::domain::{ComputedStyle, Orientation, ProgressValue};
use crate::features::vdom::domain::{
    AnchorDirection, ClickHandlers, ElementSelectors, Interactions, NodeKey, PanelSpec, UiAction,
    VNode,
};
use crate::shared::events::core::PointerButton;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{BinaryData, ModuleKey, ModuleName, ModuleOptions};

/// The key, actions and floating content every node of the table is built with.
#[derive(Debug, Clone, PartialEq)]
struct Floating {
    node_key: Option<NodeKey>,
    on_click: Option<ClickHandlers>,
    on_hover: Option<UiAction>,
    tooltip: Option<Box<StyledNode>>,
    popup: Option<StyledPopup>,
    panel: Option<StyledPanel>,
}

/// Floating content of a distinct text each, so a swapped one shows.
fn floating() -> Floating {
    let text = |content: &str| {
        let style = ComputedStyle::default();
        Box::new(text_leaf(content, style))
    };
    let panel_content = PanelSpec::new(Box::new(VNode::new_rect(
        ElementSelectors::default(),
        Interactions::default(),
    )));
    let mut clicks = ClickHandlers::new();
    clicks.insert(PointerButton::Left, UiAction::Exec("left".into()));
    clicks.insert(PointerButton::Right, UiAction::Exec("right".into()));
    Floating {
        node_key: Some(NodeKey::new("carried").unwrap()),
        on_click: Some(clicks),
        on_hover: Some(UiAction::Exec("hover".into())),
        tooltip: Some(text("tip")),
        popup: Some(StyledPopup::new(text("pop"), AnchorDirection::Bottom, None)),
        panel: Some(StyledPanel::new(text("panel"), panel_content.surface())),
    }
}

fn text_kind(parts: Floating) -> StyledNode {
    StyledNode::Text {
        path: root_path(),
        node_key: parts.node_key,
        text: TextContent::new("text".to_string()),
        style: ComputedStyle::default(),
        on_click: parts.on_click,
        on_hover: parts.on_hover,
        tooltip: parts.tooltip,
        popup: parts.popup,
        panel: parts.panel,
    }
}

fn progress_kind(parts: Floating) -> StyledNode {
    StyledNode::Progress {
        path: root_path(),
        node_key: parts.node_key,
        value: ProgressValue::new(0.5).unwrap(),
        orientation: Orientation::Horizontal,
        style: ComputedStyle::default(),
        on_click: parts.on_click,
        on_hover: parts.on_hover,
        tooltip: parts.tooltip,
        popup: parts.popup,
        panel: parts.panel,
    }
}

fn rect_kind(parts: Floating) -> StyledNode {
    StyledNode::Rect {
        path: root_path(),
        node_key: parts.node_key,
        style: ComputedStyle::default(),
        on_click: parts.on_click,
        on_hover: parts.on_hover,
        tooltip: parts.tooltip,
        popup: parts.popup,
        panel: parts.panel,
    }
}

fn image_kind(parts: Floating) -> StyledNode {
    StyledNode::Image {
        path: root_path(),
        node_key: parts.node_key,
        data: BinaryData::new(vec![0; 4]),
        pixel_size: Size::new(1, 1),
        style: ComputedStyle::default(),
        tooltip: parts.tooltip,
        popup: parts.popup,
        panel: parts.panel,
    }
}

fn module_kind(parts: Floating) -> StyledNode {
    StyledNode::Module {
        path: root_path(),
        node_key: parts.node_key,
        key: ModuleKey::from_name(ModuleName::new("clock")),
        options: ModuleOptions::default(),
        style: ComputedStyle::default(),
        on_click: parts.on_click,
        on_hover: parts.on_hover,
        tooltip: parts.tooltip,
        popup: parts.popup,
        panel: parts.panel,
    }
}

fn flex_kind(parts: Floating) -> StyledNode {
    StyledNode::Flex {
        path: root_path(),
        node_key: parts.node_key,
        children: vec![],
        style: ComputedStyle::default(),
        on_click: parts.on_click,
        on_hover: parts.on_hover,
        tooltip: parts.tooltip,
        popup: parts.popup,
        panel: parts.panel,
    }
}

fn grid_kind(parts: Floating) -> StyledNode {
    StyledNode::Grid {
        path: root_path(),
        node_key: parts.node_key,
        children: vec![],
        style: ComputedStyle::default(),
        on_click: parts.on_click,
        on_hover: parts.on_hover,
        tooltip: parts.tooltip,
        popup: parts.popup,
        panel: parts.panel,
    }
}

/// What `node` carries over, cloned.
fn floating_of(node: &RenderNode) -> Floating {
    Floating {
        node_key: node.node_key().cloned(),
        on_click: node.on_click().cloned(),
        on_hover: node.on_hover().cloned(),
        tooltip: node.tooltip().cloned().map(Box::new),
        popup: node.popup().cloned(),
        panel: node.panel().cloned(),
    }
}

/// Every kind of node keeps its key, tooltip, popup and panel, and every kind but an image,
/// which takes no actions, its click and hover actions.
#[test]
fn every_kind_of_node_keeps_its_key_actions_tooltip_popup_and_panel() {
    let kinds: [fn(Floating) -> StyledNode; 6] = [
        text_kind,
        progress_kind,
        rect_kind,
        module_kind,
        flex_kind,
        grid_kind,
    ];
    let parts = floating();
    for kind in kinds {
        let laid_out = lay_out(&mut TaffyLayoutAdapter::new(), kind(parts.clone()));
        assert_eq!(floating_of(&laid_out), parts);
    }
    let image = lay_out(&mut TaffyLayoutAdapter::new(), image_kind(parts.clone()));
    let without_actions = Floating {
        on_click: None,
        on_hover: None,
        ..parts
    };
    assert_eq!(floating_of(&image), without_actions);
}
