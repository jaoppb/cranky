use super::floating::{PanelSpec, PopupSpec};
use super::kind::VNodeKind;
use super::tag::TextContent;
use super::vnode::VNode;
use crate::features::styling::domain::{ClassName, ClassNameList};
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
pub fn substitute_failed_modules<S: std::hash::BuildHasher>(
    vnode: &VNode,
    errors: &HashMap<ModuleKey, String, S>,
) -> VNode {
    if let VNodeKind::Module {
        name, instance_id, ..
    } = &vnode.kind
    {
        let key = ModuleKey::new(name.clone(), instance_id.clone());
        if let Some(reason) = errors.get(&key) {
            return error_placeholder(vnode, name, reason);
        }
    }

    let mut result = vnode.clone();
    match &mut result.kind {
        VNodeKind::Flex { children } | VNodeKind::Grid { children } => {
            for child in children.iter_mut() {
                *child = substitute_failed_modules(child, errors);
            }
        }
        _ => {}
    }

    if let Some(tooltip) = result.tooltip.as_deref() {
        let substituted = substitute_failed_modules(tooltip, errors);
        result.tooltip = Some(Box::new(substituted));
    }
    if let Some(popup) = result.popup.as_ref() {
        let content = substitute_failed_modules(popup.content(), errors);
        result.popup = Some(
            PopupSpec::new(Box::new(content))
                .with_anchor(popup.anchor_direction())
                .with_offset(popup.offset())
                .with_dismiss_on_unfocus(popup.dismiss_on_unfocus()),
        );
    }
    if let Some(panel) = result.panel.as_ref() {
        let content = substitute_failed_modules(panel.content(), errors);
        result.panel = Some(
            PanelSpec::new(Box::new(content))
                .with_layer(panel.layer())
                .with_anchor(panel.anchor())
                .with_margin(*panel.margin())
                .with_exclusive_zone(panel.exclusive_zone())
                .with_keyboard(panel.keyboard()),
        );
    }

    result
}

fn error_placeholder(original: &VNode, name: &ModuleName, reason: &str) -> VNode {
    let mut classes: Vec<ClassName> = original
        .class_names()
        .map(|c| c.as_slice().to_vec())
        .unwrap_or_default();
    if let Ok(error_class) = ClassName::new("module-error") {
        classes.push(error_class);
    }

    VNode::new_text(
        TextContent::new(format!("{name}: {reason}")),
        Some(ClassNameList::new(classes)),
        original.element_id().cloned(),
        None,
        None,
        None,
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
            None,
            None,
            None,
            None,
            None,
        )
    }

    #[test]
    fn test_substitute_replaces_failed_module_with_error_text() {
        let tree = VNode::new_flex(
            vec![module_node("calendar")],
            None,
            None,
            None,
            None,
            None,
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
        assert!(
            children[0]
                .class_names()
                .is_some_and(|c| c.as_slice().iter().any(|c| c.as_str() == "module-error"))
        );
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
        let popup = crate::features::vdom::domain::floating::PopupSpec::new(Box::new(
            module_node("calendar"),
        ));
        let tree = VNode::new_flex(Vec::new(), None, None, None, None, None).with_popup(popup);

        let mut errors = HashMap::new();
        errors.insert(
            ModuleKey::from_name(ModuleName::new("calendar")),
            "init() failed".to_string(),
        );

        let result = substitute_failed_modules(&tree, &errors);
        let popup = result.popup().expect("popup preserved");
        assert_eq!(popup.content().tag(), NodeTag::Text);
    }
}
