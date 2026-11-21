//! The laid-out node tree that painting and pointer hit-testing work on.

mod identity;
mod interaction;
#[cfg(test)]
mod tests;

use super::popup::{StyledPanel, StyledPopup};
use super::styled_node::StyledNode;
use crate::features::styling::domain::{ComputedStyle, Orientation, ProgressValue};
use crate::features::vdom::domain::{ClickHandlers, NodeKey, NodePath, TextContent, UiAction};
use crate::shared::primitives::geometry::{Rect, Size};
use crate::shared::primitives::{BinaryData, ModuleKey, ModuleOptions};

/// A laid-out node: a `StyledNode` with its position and size resolved, ready to paint and
/// hit-test.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderNode {
    /// Flexbox container, positioned with its children.
    Flex {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
        /// Laid-out box in logical pixels, already offset by the pass's start position.
        rect: Rect,
        /// Child nodes in document order.
        children: Vec<Self>,
        /// Resolved CSS for this node.
        style: ComputedStyle,
        /// Actions to run per pointer button when the node is clicked.
        on_click: Option<ClickHandlers>,
        /// Action to run on pointer motion over the node.
        on_hover: Option<UiAction>,
        /// Tree shown in a tooltip surface while the node is hovered, not yet laid out.
        tooltip: Option<Box<StyledNode>>,
        /// Popup shown anchored to this node while present, its content not yet laid out.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present, its content not yet laid out.
        panel: Option<StyledPanel>,
    },
    /// CSS grid container, positioned with its children.
    Grid {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
        /// Laid-out box in logical pixels, already offset by the pass's start position.
        rect: Rect,
        /// Child nodes in document order.
        children: Vec<Self>,
        /// Resolved CSS for this node.
        style: ComputedStyle,
        /// Actions to run per pointer button when the node is clicked.
        on_click: Option<ClickHandlers>,
        /// Action to run on pointer motion over the node.
        on_hover: Option<UiAction>,
        /// Tree shown in a tooltip surface while the node is hovered, not yet laid out.
        tooltip: Option<Box<StyledNode>>,
        /// Popup shown anchored to this node while present, its content not yet laid out.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present, its content not yet laid out.
        panel: Option<StyledPanel>,
    },
    /// Text leaf, sized by the text measurer.
    Text {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
        /// Laid-out box in logical pixels, already offset by the pass's start position.
        rect: Rect,
        /// The string to shape and draw.
        text: TextContent,
        /// Resolved CSS for this node.
        style: ComputedStyle,
        /// Actions to run per pointer button when the node is clicked.
        on_click: Option<ClickHandlers>,
        /// Action to run on pointer motion over the node.
        on_hover: Option<UiAction>,
        /// Tree shown in a tooltip surface while the node is hovered, not yet laid out.
        tooltip: Option<Box<StyledNode>>,
        /// Popup shown anchored to this node while present, its content not yet laid out.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present, its content not yet laid out.
        panel: Option<StyledPanel>,
    },
    /// Progress bar leaf, filled along `orientation` in proportion to `value`.
    Progress {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
        /// Laid-out box in logical pixels, already offset by the pass's start position.
        rect: Rect,
        /// Fill fraction, 0.0 to 1.0.
        value: ProgressValue,
        /// Axis along which the bar fills.
        orientation: Orientation,
        /// Resolved CSS for this node.
        style: ComputedStyle,
        /// Actions to run per pointer button when the node is clicked.
        on_click: Option<ClickHandlers>,
        /// Action to run on pointer motion over the node.
        on_hover: Option<UiAction>,
        /// Tree shown in a tooltip surface while the node is hovered, not yet laid out.
        tooltip: Option<Box<StyledNode>>,
        /// Popup shown anchored to this node while present, its content not yet laid out.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present, its content not yet laid out.
        panel: Option<StyledPanel>,
    },
    /// Plain box filled with its background, or its `color` if it has none, plus its border.
    Rect {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
        /// Laid-out box in logical pixels, already offset by the pass's start position.
        rect: Rect,
        /// Resolved CSS for this node.
        style: ComputedStyle,
        /// Actions to run per pointer button when the node is clicked.
        on_click: Option<ClickHandlers>,
        /// Action to run on pointer motion over the node.
        on_hover: Option<UiAction>,
        /// Tree shown in a tooltip surface while the node is hovered, not yet laid out.
        tooltip: Option<Box<StyledNode>>,
        /// Popup shown anchored to this node while present, its content not yet laid out.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present, its content not yet laid out.
        panel: Option<StyledPanel>,
    },
    /// Image leaf; it carries no style and has no click or hover actions.
    Image {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
        /// Laid-out box in logical pixels, already offset by the pass's start position.
        rect: Rect,
        /// Straight-alpha RGBA pixels, `pixel_size` in size.
        data: BinaryData,
        /// Size of `data` in pixels; the image is scaled to the laid-out box.
        pixel_size: Size,
        /// Tree shown in a tooltip surface while the node is hovered, not yet laid out.
        tooltip: Option<Box<StyledNode>>,
        /// Popup shown anchored to this node while present, its content not yet laid out.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present, its content not yet laid out.
        panel: Option<StyledPanel>,
    },
    /// Box reserved for an embedded child module; only its background and border paint here.
    Module {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
        /// Laid-out box in logical pixels, already offset by the pass's start position.
        rect: Rect,
        /// Name and instance of the embedded child module.
        key: ModuleKey,
        /// Options passed to the embedded child module.
        options: ModuleOptions,
        /// Resolved CSS for this node.
        style: ComputedStyle,
        /// Actions to run per pointer button when the node is clicked.
        on_click: Option<ClickHandlers>,
        /// Action to run on pointer motion over the node.
        on_hover: Option<UiAction>,
        /// Tree shown in a tooltip surface while the node is hovered, not yet laid out.
        tooltip: Option<Box<StyledNode>>,
        /// Popup shown anchored to this node while present, its content not yet laid out.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present, its content not yet laid out.
        panel: Option<StyledPanel>,
    },
}
