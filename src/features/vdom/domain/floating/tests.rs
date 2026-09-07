use super::{
    AnchorDirection, ExclusiveZone, KeyboardInteractivity, PanelAnchor, PanelLayer, PanelSpec,
    PopupOffset, PopupSpec,
};
use crate::features::vdom::domain::{TextContent, VNode};

#[test]
fn test_popup_spec_builder_and_accessors() {
    let content = Box::new(VNode::new_text(
        TextContent::new("popup".to_string()),
        None,
        None,
        None,
        None,
        None,
    ));
    let spec = PopupSpec::new(content.clone())
        .with_anchor(AnchorDirection::Bottom)
        .with_offset(Some(PopupOffset::new(0, 10)))
        .with_dismiss_on_unfocus(false);

    assert_eq!(spec.content(), content.as_ref());
    assert_eq!(spec.anchor_direction(), AnchorDirection::Bottom);
    assert_eq!(spec.offset(), Some(PopupOffset::new(0, 10)));
    assert!(!spec.dismiss_on_unfocus());
}

#[test]
fn test_panel_spec_builder_and_accessors() {
    let content = Box::new(VNode::new_text(
        TextContent::new("panel".to_string()),
        None,
        None,
        None,
        None,
        None,
    ));
    let anchor = PanelAnchor::new(true, false, false, true);
    let spec = PanelSpec::new(content.clone())
        .with_layer(PanelLayer::Overlay)
        .with_anchor(anchor)
        .with_exclusive_zone(ExclusiveZone::auto())
        .with_keyboard(KeyboardInteractivity::Exclusive);

    assert_eq!(spec.content(), content.as_ref());
    assert_eq!(spec.layer(), PanelLayer::Overlay);
    assert_eq!(spec.anchor(), anchor);
    assert_eq!(spec.exclusive_zone(), ExclusiveZone::auto());
    assert_eq!(spec.keyboard(), KeyboardInteractivity::Exclusive);
}
