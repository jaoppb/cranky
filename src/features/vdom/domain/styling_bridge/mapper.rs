use crate::features::layout_engine::domain::{StyledNode, StyledPanel, StyledPopup};
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::{NodePath, VNode, VNodeKind};
use crate::shared::primitives::ModuleKey;

#[must_use]
pub fn map_to_styled_node(
    vnode: &VNode,
    path: &NodePath,
    style: ComputedStyle,
    tooltip: Option<Box<StyledNode>>,
    popup: Option<StyledPopup>,
    panel: Option<StyledPanel>,
    children: Vec<StyledNode>,
) -> StyledNode {
    let on_click = vnode.on_click().cloned();
    let on_hover = vnode.on_hover().cloned();
    let path = path.clone();

    match vnode.kind() {
        VNodeKind::Flex { .. } => StyledNode::Flex {
            path,
            children,
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
            panel,
        },
        VNodeKind::Grid { .. } => StyledNode::Grid {
            path,
            children,
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
            panel,
        },
        VNodeKind::Text { text } => StyledNode::Text {
            path,
            text: text.clone(),
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
            panel,
        },
        VNodeKind::Progress { value, orientation } => StyledNode::Progress {
            path,
            value: *value,
            orientation: *orientation,
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
            panel,
        },
        VNodeKind::Rect => StyledNode::Rect {
            path,
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
            panel,
        },
        VNodeKind::Image { data, pixel_size } => StyledNode::Image {
            path,
            data: data.clone(),
            pixel_size: *pixel_size,
            style,
            tooltip,
            popup,
            panel,
        },
        VNodeKind::Module {
            name,
            instance_id,
            options,
        } => StyledNode::Module {
            path,
            key: ModuleKey::new(name.clone(), instance_id.clone()),
            options: options.clone(),
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
            panel,
        },
    }
}
