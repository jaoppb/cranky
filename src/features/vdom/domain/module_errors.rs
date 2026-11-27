//! Swaps embedded `Module` nodes whose lazy spawn failed for a text node showing the error.
use super::floating::{PanelSpec, PopupSpec};
use super::kind::VNodeKind;
use super::tag::TextContent;
use super::vnode::VNode;
use crate::features::styling::domain::{ClassName, ClassNameList};
use crate::features::vdom::domain::{ElementSelectors, Interactions};
use crate::shared::primitives::{ModuleKey, ModuleName};
use std::collections::HashMap;

/// Replaces every `Module` node whose site has a recorded lazy-spawn failure
/// with a plain text node styled `.module-error`, carrying the module's name
/// and the failure reason.
///
/// Runs once, on the raw script output, right before style resolution — so
/// the error node gets sized and painted like any other node instead of
/// needing its own render path. Walks children, tooltip, popup and panel
/// content: a failed module can sit anywhere in the tree, including inside
/// a popup's own content, which is the case this feature exists for.
#[must_use]
pub(crate) fn substitute_failed_modules<S: std::hash::BuildHasher>(
    vnode: &VNode,
    errors: &HashMap<ModuleKey, String, S>,
) -> VNode {
    if let Some(placeholder) = placeholder_if_failed(vnode, errors) {
        return placeholder;
    }

    let mut result = vnode.clone();
    substitute_in_children(&mut result.kind, errors);
    result.tooltip = result
        .tooltip
        .as_deref()
        .map(|tooltip| Box::new(substitute_failed_modules(tooltip, errors)));
    result.popup = result
        .popup
        .as_ref()
        .map(|popup| substitute_in_popup(popup, errors));
    result.panel = result
        .panel
        .as_ref()
        .map(|panel| substitute_in_panel(panel, errors));
    result
}

/// The error placeholder for `vnode` when it is a `Module` node whose site has a recorded
/// failure, `None` otherwise.
fn placeholder_if_failed<S: std::hash::BuildHasher>(
    vnode: &VNode,
    errors: &HashMap<ModuleKey, String, S>,
) -> Option<VNode> {
    let VNodeKind::Module {
        name, instance_id, ..
    } = &vnode.kind
    else {
        return None;
    };
    let key = ModuleKey::new(name.clone(), instance_id.clone());
    errors
        .get(&key)
        .map(|reason| error_placeholder(vnode, name, reason))
}

/// Substitutes inside each child of a flex or grid `kind`; other kinds have no children.
fn substitute_in_children<S: std::hash::BuildHasher>(
    kind: &mut VNodeKind,
    errors: &HashMap<ModuleKey, String, S>,
) {
    if let VNodeKind::Flex { children } | VNodeKind::Grid { children } = kind {
        for child in children.iter_mut() {
            *child = substitute_failed_modules(child, errors);
        }
    }
}

/// `popup` with its content substituted, keeping its anchor and offset.
fn substitute_in_popup<S: std::hash::BuildHasher>(
    popup: &PopupSpec,
    errors: &HashMap<ModuleKey, String, S>,
) -> PopupSpec {
    let content = substitute_failed_modules(popup.content(), errors);
    PopupSpec::new(Box::new(content))
        .with_anchor(popup.anchor_direction())
        .with_offset(popup.offset())
}

/// `panel` with its content substituted, keeping its layer, anchor, margin, exclusive zone and
/// keyboard interactivity.
fn substitute_in_panel<S: std::hash::BuildHasher>(
    panel: &PanelSpec,
    errors: &HashMap<ModuleKey, String, S>,
) -> PanelSpec {
    let content = substitute_failed_modules(panel.content(), errors);
    PanelSpec::new(Box::new(content))
        .with_layer(panel.layer())
        .with_anchor(panel.anchor())
        .with_margin(*panel.margin())
        .with_exclusive_zone(panel.exclusive_zone())
        .with_keyboard(panel.keyboard())
}

fn error_placeholder(original: &VNode, name: &ModuleName, reason: &str) -> VNode {
    let mut classes: Vec<ClassName> = original
        .class_names()
        .map_or_default(|class_list| class_list.as_slice().to_vec());
    if let Ok(error_class) = ClassName::new("module-error") {
        classes.push(error_class);
    }

    VNode::new_text(
        TextContent::new(format!("{name}: {reason}")),
        ElementSelectors::new(
            Some(ClassNameList::new(classes)),
            original.element_id().cloned(),
        ),
        Interactions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::vdom::domain::kind::ModuleParams;
    use crate::features::vdom::domain::tag::NodeTag;

    fn module_node(name: &str) -> VNode {
        VNode::new_module(
            ModuleParams::new(
                ModuleName::new(name),
                None,
                crate::shared::primitives::ModuleOptions::default(),
            ),
            ElementSelectors::default(),
            Interactions::default(),
        )
    }

    #[test]
    fn test_substitute_replaces_failed_module_with_error_text() {
        let tree = VNode::new_flex(
            vec![module_node("calendar")],
            ElementSelectors::default(),
            Interactions::default(),
        );
        let mut errors = HashMap::new();
        errors.insert(
            ModuleKey::from_name(ModuleName::new("calendar")),
            "not found".to_string(),
        );

        let result = substitute_failed_modules(&tree, &errors);
        let VNodeKind::Flex { children } = result.kind() else {
            panic!("expected flex root to survive substitution");
        };
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].tag(), NodeTag::Text);
        let VNodeKind::Text { text } = children[0].kind() else {
            panic!("expected substituted child to be text");
        };
        assert_eq!(text.as_str(), "calendar: not found");
        assert!(children[0].class_names().is_some_and(|class_list| {
            class_list
                .as_slice()
                .iter()
                .any(|class| class.as_str() == "module-error")
        }));
    }

    #[test]
    fn test_substitute_leaves_unrelated_modules_untouched() {
        let tree = module_node("clock");
        let mut errors = HashMap::new();
        errors.insert(
            ModuleKey::from_name(ModuleName::new("calendar")),
            "not found".to_string(),
        );

        let result = substitute_failed_modules(&tree, &errors);
        assert_eq!(result.tag(), NodeTag::Module);
    }

    #[test]
    fn test_substitute_recurses_into_popup_content() {
        let popup = crate::features::vdom::domain::floating::PopupSpec::new(Box::new(module_node(
            "calendar",
        )));
        let tree = VNode::new_flex(
            Vec::new(),
            ElementSelectors::default(),
            Interactions::default(),
        )
        .with_popup(popup);

        let mut errors = HashMap::new();
        errors.insert(
            ModuleKey::from_name(ModuleName::new("calendar")),
            "init() failed".to_string(),
        );

        let result = substitute_failed_modules(&tree, &errors);
        let popup = result.popup().expect("popup preserved");
        assert_eq!(popup.content().tag(), NodeTag::Text);
    }

    /// A failure recorded for the `calendar` module without an instance id.
    fn calendar_not_found() -> HashMap<ModuleKey, String> {
        HashMap::from([(
            ModuleKey::from_name(ModuleName::new("calendar")),
            "not found".to_string(),
        )])
    }

    /// The text of `node` when it is a text node.
    fn text_of(node: &VNode) -> Option<&str> {
        match node.kind() {
            VNodeKind::Text { text } => Some(text.as_str()),
            _ => None,
        }
    }

    fn text_node(label: &str) -> VNode {
        VNode::new_text(
            TextContent::new(label.to_string()),
            ElementSelectors::default(),
            Interactions::default(),
        )
    }

    #[test]
    fn test_substitute_recurses_into_grid_children_and_tooltip() {
        let tree = VNode::new_grid(
            vec![module_node("clock"), module_node("calendar")],
            ElementSelectors::default(),
            Interactions::new(None, None, Some(Box::new(module_node("calendar")))),
        );
        let result = substitute_failed_modules(&tree, &calendar_not_found());
        let children: Vec<NodeTag> = result.children().iter().map(VNode::tag).collect();
        assert_eq!(children, vec![NodeTag::Module, NodeTag::Text]);
        assert_eq!(text_of(&result.children()[1]), Some("calendar: not found"));
        assert_eq!(
            result.tooltip().and_then(text_of),
            Some("calendar: not found")
        );
    }

    fn panel_margin() -> crate::shared::config::domain::MarginConfig {
        use crate::shared::config::domain::{AxisMargins, MarginConfig, MarginOffset};
        MarginConfig::new(
            AxisMargins::new(MarginOffset::new(1), MarginOffset::new(2)),
            AxisMargins::new(MarginOffset::new(3), MarginOffset::new(4)),
        )
    }

    /// A panel around `content` with every placement field away from its default.
    fn placed_panel(content: VNode) -> PanelSpec {
        use crate::features::vdom::domain::{
            ExclusiveZone, KeyboardInteractivity, PanelAnchor, PanelLayer,
        };
        PanelSpec::new(Box::new(content))
            .with_layer(PanelLayer::Overlay)
            .with_anchor(PanelAnchor::all())
            .with_margin(panel_margin())
            .with_exclusive_zone(ExclusiveZone::new(7))
            .with_keyboard(KeyboardInteractivity::Exclusive)
    }

    /// A popup around `content` anchored to the top with an offset.
    fn placed_popup(content: VNode) -> PopupSpec {
        use crate::features::vdom::domain::{AnchorDirection, PopupOffset};
        PopupSpec::new(Box::new(content))
            .with_anchor(AnchorDirection::Top)
            .with_offset(Some(PopupOffset::new(3, 4)))
    }

    #[test]
    fn test_substitute_recurses_into_panel_content_and_keeps_its_placement() {
        let tree = text_node("owner").with_panel(placed_panel(module_node("calendar")));
        let result = substitute_failed_modules(&tree, &calendar_not_found());
        let panel = result.panel().unwrap();
        let expected = placed_panel(text_node("unused"));
        assert_eq!(text_of(panel.content()), Some("calendar: not found"));
        assert_eq!(
            (
                panel.layer(),
                panel.anchor(),
                panel.margin(),
                panel.exclusive_zone()
            ),
            (
                expected.layer(),
                expected.anchor(),
                expected.margin(),
                expected.exclusive_zone()
            )
        );
        assert_eq!(panel.keyboard(), expected.keyboard());
    }

    #[test]
    fn test_substitute_recurses_into_popup_content_and_keeps_its_placement() {
        let tree = text_node("owner").with_popup(placed_popup(module_node("calendar")));
        let result = substitute_failed_modules(&tree, &calendar_not_found());
        let popup = result.popup().unwrap();
        assert_eq!(text_of(popup.content()), Some("calendar: not found"));
        let expected = placed_popup(text_node("unused"));
        assert_eq!(
            (popup.anchor_direction(), popup.offset()),
            (expected.anchor_direction(), expected.offset())
        );
    }

    #[test]
    fn test_substitute_without_failures_returns_an_equal_tree() {
        let tree = VNode::new_flex(
            vec![module_node("clock"), text_node("label")],
            ElementSelectors::default(),
            Interactions::new(None, None, Some(Box::new(text_node("tip")))),
        )
        .with_popup(placed_popup(module_node("clock")))
        .with_panel(placed_panel(module_node("clock")));
        assert_eq!(
            substitute_failed_modules(&tree, &calendar_not_found()),
            tree
        );
    }

    /// A failed `calendar` module carrying classes, an id, a key, a click, a hover, a tooltip and a
    /// popup.
    fn decorated_failed_calendar() -> VNode {
        use crate::features::styling::domain::ElementId;
        use crate::features::vdom::domain::{ClickHandlers, NodeKey, UiAction};
        VNode::new_module(
            ModuleParams::new(
                ModuleName::new("calendar"),
                None,
                crate::shared::primitives::ModuleOptions::default(),
            ),
            ElementSelectors::new(
                Some(ClassNameList::parse("slot wide").unwrap()),
                Some(ElementId::new("cal").unwrap()),
            ),
            Interactions::new(
                Some(ClickHandlers::from_single(UiAction::Exec("open".into()))),
                Some(UiAction::Exec("hover".into())),
                Some(Box::new(text_node("tip"))),
            ),
        )
        .with_key(NodeKey::new("cal").unwrap())
        .with_popup(placed_popup(text_node("pop")))
    }

    #[test]
    fn test_placeholder_keeps_classes_and_id_and_drops_everything_else() {
        use crate::features::styling::domain::ElementId;
        let failed = decorated_failed_calendar();
        let result = substitute_failed_modules(&failed, &calendar_not_found());
        assert_eq!(
            (result.class_names(), result.element_id()),
            (
                Some(&ClassNameList::parse("slot wide module-error").unwrap()),
                Some(&ElementId::new("cal").unwrap())
            )
        );
        assert_eq!(
            (result.on_click(), result.on_hover(), result.tooltip()),
            (None, None, None)
        );
        assert_eq!(
            (result.popup(), result.panel(), result.key()),
            (None, None, None)
        );
    }

    #[test]
    fn test_substitute_matches_the_instance_id_of_the_failed_site() {
        use crate::shared::primitives::{ModuleInstanceId, ModuleOptions};
        let instance = |id: Option<&str>| {
            let params = ModuleParams::new(
                ModuleName::new("calendar"),
                id.map(ModuleInstanceId::new),
                ModuleOptions::default(),
            );
            VNode::new_module(params, ElementSelectors::default(), Interactions::default())
        };
        let errors = HashMap::from([(
            ModuleKey::new(
                ModuleName::new("calendar"),
                Some(ModuleInstanceId::new("left")),
            ),
            "boom".to_string(),
        )]);
        let tree = VNode::new_flex(
            vec![instance(None), instance(Some("left"))],
            ElementSelectors::default(),
            Interactions::default(),
        );
        let result = substitute_failed_modules(&tree, &errors);
        let texts: Vec<Option<&str>> = result.children().iter().map(text_of).collect();
        assert_eq!(texts, vec![None, Some("calendar: boom")]);
    }
}
