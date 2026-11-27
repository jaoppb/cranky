//! Constructors that build a `VNode` of each kind with a fresh `NodeId`, no key and no popup or
//! panel.
pub mod attributes;

pub use attributes::{ElementSelectors, Interactions};
pub(crate) use attributes::{ImageContent, ProgressBar};

use super::identifier::NodeId;
use super::kind::{ModuleParams, VNodeKind};
use super::tag::TextContent;
use super::vnode::VNode;

impl VNode {
    /// A node of `kind` with `selectors` and `interactions`, a fresh `NodeId`, no key and no popup
    /// or panel.
    fn from_kind(kind: VNodeKind, selectors: ElementSelectors, interactions: Interactions) -> Self {
        Self {
            node_id: NodeId::new(),
            key: None,
            id: selectors.id,
            class: selectors.class,
            on_click: interactions.on_click,
            on_hover: interactions.on_hover,
            tooltip: interactions.tooltip,
            popup: None,
            panel: None,
            kind,
        }
    }

    /// Builds a flex container node, with a fresh `NodeId`, no key and no popup or panel.
    #[must_use]
    pub fn new_flex(
        children: Vec<Self>,
        selectors: ElementSelectors,
        interactions: Interactions,
    ) -> Self {
        Self::from_kind(VNodeKind::Flex { children }, selectors, interactions)
    }

    #[must_use]
    pub(crate) fn new_grid(
        children: Vec<Self>,
        selectors: ElementSelectors,
        interactions: Interactions,
    ) -> Self {
        Self::from_kind(VNodeKind::Grid { children }, selectors, interactions)
    }

    /// Builds a text node, with a fresh `NodeId`, no key and no popup or panel.
    #[must_use]
    pub fn new_text(
        text: TextContent,
        selectors: ElementSelectors,
        interactions: Interactions,
    ) -> Self {
        Self::from_kind(VNodeKind::Text { text }, selectors, interactions)
    }

    #[must_use]
    pub(crate) fn new_progress(
        bar: ProgressBar,
        selectors: ElementSelectors,
        interactions: Interactions,
    ) -> Self {
        let kind = VNodeKind::Progress {
            value: bar.value,
            orientation: bar.orientation,
        };
        Self::from_kind(kind, selectors, interactions)
    }

    #[must_use]
    pub(crate) fn new_rect(selectors: ElementSelectors, interactions: Interactions) -> Self {
        Self::from_kind(VNodeKind::Rect, selectors, interactions)
    }

    /// An image reacts to no click or hover, so it takes only a tooltip.
    #[must_use]
    pub(crate) fn new_image(
        image: ImageContent,
        selectors: ElementSelectors,
        tooltip: Option<Box<Self>>,
    ) -> Self {
        let kind = VNodeKind::Image {
            data: image.data,
            pixel_size: image.pixel_size,
        };
        Self::from_kind(kind, selectors, Interactions::new(None, None, tooltip))
    }

    #[must_use]
    pub(crate) fn new_module(
        params: ModuleParams,
        selectors: ElementSelectors,
        interactions: Interactions,
    ) -> Self {
        let (name, instance_id, options) = params.into_parts();
        let kind = VNodeKind::Module {
            name,
            instance_id,
            options,
        };
        Self::from_kind(kind, selectors, interactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::{ClassNameList, ElementId, Orientation, ProgressValue};
    use crate::features::vdom::domain::{ClickHandlers, UiAction};
    use crate::shared::events::core::PointerButton;
    use crate::shared::primitives::BinaryData;
    use crate::shared::primitives::geometry::Size;
    use crate::shared::primitives::{DynamicValue, ModuleInstanceId, ModuleName, ModuleOptions};
    use std::collections::HashMap;

    fn class(label: &str) -> ClassNameList {
        ClassNameList::parse(&format!("{label}-class")).unwrap()
    }

    fn element_id(label: &str) -> ElementId {
        ElementId::new(format!("{label}-id")).unwrap()
    }

    fn on_click(label: &str) -> ClickHandlers {
        let mut handlers = ClickHandlers::new();
        handlers.insert(
            PointerButton::Right,
            UiAction::Exec(format!("{label}-click")),
        );
        handlers
    }

    fn on_hover(label: &str) -> UiAction {
        UiAction::Exec(format!("{label}-hover"))
    }

    fn tooltip(label: &str) -> Box<VNode> {
        Box::new(VNode::new_text(
            TextContent::new(format!("{label}-tip")),
            ElementSelectors::default(),
            Interactions::default(),
        ))
    }

    /// The class and id made from `label`.
    fn selectors(label: &str) -> ElementSelectors {
        ElementSelectors::new(Some(class(label)), Some(element_id(label)))
    }

    /// The click, hover and tooltip made from `label`.
    fn interactions(label: &str) -> Interactions {
        Interactions::new(
            Some(on_click(label)),
            Some(on_hover(label)),
            Some(tooltip(label)),
        )
    }

    /// Asserts `node` carries the class, id and tooltip made from `label`, and its click and
    /// hover handlers when `handlers` names it.
    fn assert_attributes(node: &VNode, label: &str, handlers: Option<&str>) {
        let expected_selectors = (Some(&class(label)), Some(&element_id(label)));
        assert_eq!((node.class_names(), node.element_id()), expected_selectors);
        let expected_handlers = (handlers.map(on_click), handlers.map(on_hover));
        let actual_handlers = (node.on_click().cloned(), node.on_hover().cloned());
        assert_eq!(actual_handlers, expected_handlers);
        assert_eq!(node.tooltip().map(VNode::kind), Some(tooltip(label).kind()));
        assert_eq!((node.key(), node.popup(), node.panel()), (None, None, None));
    }

    #[test]
    fn test_container_constructors_keep_each_attribute_in_place() {
        let child = VNode::new_rect(ElementSelectors::default(), Interactions::default());
        let flex = VNode::new_flex(vec![child.clone()], selectors("flex"), interactions("flex"));
        assert_attributes(&flex, "flex", Some("flex"));
        assert_eq!(
            flex.kind(),
            &VNodeKind::Flex {
                children: vec![child.clone()]
            }
        );

        let grid = VNode::new_grid(
            vec![child.clone(), child],
            selectors("grid"),
            interactions("grid"),
        );
        assert_attributes(&grid, "grid", Some("grid"));
        assert_eq!(grid.children().len(), 2);
        assert!(matches!(grid.kind(), VNodeKind::Grid { .. }));
    }

    #[test]
    fn test_leaf_constructors_keep_each_attribute_in_place() {
        let hello = TextContent::new("hello".to_string());
        let text = VNode::new_text(hello.clone(), selectors("text"), interactions("text"));
        assert_attributes(&text, "text", Some("text"));
        assert_eq!(text.kind(), &VNodeKind::Text { text: hello });

        let value = ProgressValue::new(0.3).unwrap();
        let bar = ProgressBar::new(value, Orientation::Vertical);
        let progress = VNode::new_progress(bar, selectors("bar"), interactions("bar"));
        assert_attributes(&progress, "bar", Some("bar"));
        let orientation = Orientation::Vertical;
        assert_eq!(progress.kind(), &VNodeKind::Progress { value, orientation });

        let rect = VNode::new_rect(selectors("rect"), interactions("rect"));
        assert_attributes(&rect, "rect", Some("rect"));
        assert_eq!(rect.kind(), &VNodeKind::Rect);
    }

    #[test]
    fn test_image_and_module_constructors_keep_each_attribute_in_place() {
        let image_content = ImageContent::new(vec![1, 2, 3, 4], Size::new(1, 1));
        let image = VNode::new_image(image_content, selectors("icon"), Some(tooltip("icon")));
        assert_attributes(&image, "icon", None);
        let data = BinaryData::new(vec![1, 2, 3, 4]);
        let pixel_size = Size::new(1, 1);
        assert_eq!(image.kind(), &VNodeKind::Image { data, pixel_size });

        let options = ModuleOptions::new(HashMap::from([(
            "format".to_string(),
            DynamicValue::String("%H".to_string()),
        )]));
        let (name, instance_id) = (
            ModuleName::new("clock"),
            Some(ModuleInstanceId::new("left")),
        );
        let params = ModuleParams::new(name.clone(), instance_id.clone(), options.clone());
        let module = VNode::new_module(params, selectors("clock"), interactions("clock"));
        assert_attributes(&module, "clock", Some("clock"));
        let expected = VNodeKind::Module {
            name,
            instance_id,
            options,
        };
        assert_eq!(module.kind(), &expected);
    }
}
