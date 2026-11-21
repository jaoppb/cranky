//! The styled node tree that a layout pass consumes.

mod accessors;

use super::popup::{StyledPanel, StyledPopup};
use crate::features::styling::domain::{ComputedStyle, Orientation, ProgressValue};
use crate::features::vdom::domain::{ClickHandlers, NodeKey, NodePath, TextContent, UiAction};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{BinaryData, ModuleKey, ModuleOptions};

/// A widget node with its CSS resolved but no geometry yet; the input to a layout pass.
#[derive(Debug, Clone, PartialEq)]
pub enum StyledNode {
    /// Flexbox container.
    Flex {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
        /// Child nodes in document order.
        children: Vec<Self>,
        /// Resolved CSS for this node.
        style: ComputedStyle,
        /// Actions to run per pointer button when the node is clicked.
        on_click: Option<ClickHandlers>,
        /// Action to run on pointer motion over the node.
        on_hover: Option<UiAction>,
        /// Tree shown in a tooltip surface while the node is hovered.
        tooltip: Option<Box<Self>>,
        /// Popup shown anchored to this node while present in the tree.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present in the tree.
        panel: Option<StyledPanel>,
    },
    /// CSS grid container.
    Grid {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
        /// Child nodes in document order.
        children: Vec<Self>,
        /// Resolved CSS for this node.
        style: ComputedStyle,
        /// Actions to run per pointer button when the node is clicked.
        on_click: Option<ClickHandlers>,
        /// Action to run on pointer motion over the node.
        on_hover: Option<UiAction>,
        /// Tree shown in a tooltip surface while the node is hovered.
        tooltip: Option<Box<Self>>,
        /// Popup shown anchored to this node while present in the tree.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present in the tree.
        panel: Option<StyledPanel>,
    },
    /// Text leaf, sized by the text measurer.
    Text {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
        /// The string to shape and draw.
        text: TextContent,
        /// Resolved CSS for this node.
        style: ComputedStyle,
        /// Actions to run per pointer button when the node is clicked.
        on_click: Option<ClickHandlers>,
        /// Action to run on pointer motion over the node.
        on_hover: Option<UiAction>,
        /// Tree shown in a tooltip surface while the node is hovered.
        tooltip: Option<Box<Self>>,
        /// Popup shown anchored to this node while present in the tree.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present in the tree.
        panel: Option<StyledPanel>,
    },
    /// Progress bar leaf, filled along `orientation` in proportion to `value`.
    Progress {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
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
        /// Tree shown in a tooltip surface while the node is hovered.
        tooltip: Option<Box<Self>>,
        /// Popup shown anchored to this node while present in the tree.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present in the tree.
        panel: Option<StyledPanel>,
    },
    /// Plain box with no content, filled with its background, or its `color` if it has none, plus
    /// its border.
    Rect {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
        /// Resolved CSS for this node.
        style: ComputedStyle,
        /// Actions to run per pointer button when the node is clicked.
        on_click: Option<ClickHandlers>,
        /// Action to run on pointer motion over the node.
        on_hover: Option<UiAction>,
        /// Tree shown in a tooltip surface while the node is hovered.
        tooltip: Option<Box<Self>>,
        /// Popup shown anchored to this node while present in the tree.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present in the tree.
        panel: Option<StyledPanel>,
    },
    /// Image leaf; it has no click or hover actions.
    Image {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
        /// Straight-alpha RGBA pixels, `pixel_size` in size.
        data: BinaryData,
        /// Size of `data` in pixels, before scaling to the laid-out box.
        pixel_size: Size,
        /// Resolved CSS for this node.
        style: ComputedStyle,
        /// Tree shown in a tooltip surface while the node is hovered.
        tooltip: Option<Box<Self>>,
        /// Popup shown anchored to this node while present in the tree.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present in the tree.
        panel: Option<StyledPanel>,
    },
    /// Embedded child module, sized from the size it last reported.
    Module {
        /// Positional path of this node within its surface's tree.
        path: NodePath,
        /// Script-set reconciliation key, if the widget opted in.
        node_key: Option<NodeKey>,
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
        /// Tree shown in a tooltip surface while the node is hovered.
        tooltip: Option<Box<Self>>,
        /// Popup shown anchored to this node while present in the tree.
        popup: Option<StyledPopup>,
        /// Layer-shell panel shown while present in the tree.
        panel: Option<StyledPanel>,
    },
}
