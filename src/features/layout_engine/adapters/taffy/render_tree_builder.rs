use crate::features::layout_engine::domain::{LayoutError, RenderNode, StyledNode};
use crate::shared::primitives::geometry::{Position, Rect, Size};
use taffy::tree::NodeId;
use taffy::TaffyTree;

#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
pub(super) fn build_render_tree(
    taffy: &TaffyTree,
    node_id: NodeId,
    node: &StyledNode,
    offset: Position,
) -> Result<RenderNode, LayoutError> {
    let layout = taffy
        .layout(node_id)
        .map_err(|e| LayoutError::EngineError(e.to_string()))?;

    let abs_x = offset.x().saturating_add(layout.location.x as i32);
    let abs_y = offset.y().saturating_add(layout.location.y as i32);
    let rect = Rect::new(
        Position::new(abs_x, abs_y),
        Size::new(layout.size.width as u32, layout.size.height as u32),
    );

    match node {
        StyledNode::Flex {
            path,
            children,
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
        } => {
            let child_ids = taffy
                .children(node_id)
                .map_err(|e| LayoutError::EngineError(e.to_string()))?;
            let mut render_children = Vec::new();

            for (child, &child_id) in children.iter().zip(child_ids.iter()) {
                render_children.push(build_render_tree(
                    taffy,
                    child_id,
                    child,
                    Position::new(abs_x, abs_y),
                )?);
            }

            Ok(RenderNode::Flex {
                path: path.clone(),
                rect,
                children: render_children,
                style: style.clone(),
                on_click: on_click.clone(),
                on_hover: on_hover.clone(),
                tooltip: tooltip.clone(),
                popup: popup.clone(),
            })
        }
        StyledNode::Grid {
            path,
            children,
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
        } => {
            let child_ids = taffy
                .children(node_id)
                .map_err(|e| LayoutError::EngineError(e.to_string()))?;
            let mut render_children = Vec::new();

            for (child, &child_id) in children.iter().zip(child_ids.iter()) {
                render_children.push(build_render_tree(
                    taffy,
                    child_id,
                    child,
                    Position::new(abs_x, abs_y),
                )?);
            }

            Ok(RenderNode::Grid {
                path: path.clone(),
                rect,
                children: render_children,
                style: style.clone(),
                on_click: on_click.clone(),
                on_hover: on_hover.clone(),
                tooltip: tooltip.clone(),
                popup: popup.clone(),
            })
        }
        StyledNode::Text {
            path,
            text,
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
        } => Ok(RenderNode::Text {
            path: path.clone(),
            rect,
            text: text.clone(),
            style: style.clone(),
            on_click: on_click.clone(),
            on_hover: on_hover.clone(),
            tooltip: tooltip.clone(),
            popup: popup.clone(),
        }),
        StyledNode::Progress {
            path,
            value,
            orientation,
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
        } => Ok(RenderNode::Progress {
            path: path.clone(),
            rect,
            value: *value,
            orientation: *orientation,
            style: style.clone(),
            on_click: on_click.clone(),
            on_hover: on_hover.clone(),
            tooltip: tooltip.clone(),
            popup: popup.clone(),
        }),
        StyledNode::Rect {
            path,
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
        } => Ok(RenderNode::Rect {
            path: path.clone(),
            rect,
            style: style.clone(),
            on_click: on_click.clone(),
            on_hover: on_hover.clone(),
            tooltip: tooltip.clone(),
            popup: popup.clone(),
        }),
        StyledNode::Image {
            path,
            data,
            pixel_size,
            tooltip,
            popup,
            ..
        } => Ok(RenderNode::Image {
            path: path.clone(),
            rect,
            data: data.clone(),
            pixel_size: *pixel_size,
            tooltip: tooltip.clone(),
            popup: popup.clone(),
        }),
        StyledNode::Module {
            path,
            key,
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
            ..
        } => Ok(RenderNode::Module {
            path: path.clone(),
            rect,
            key: key.clone(),
            style: style.clone(),
            on_click: on_click.clone(),
            on_hover: on_hover.clone(),
            tooltip: tooltip.clone(),
            popup: popup.clone(),
        }),
    }
}
