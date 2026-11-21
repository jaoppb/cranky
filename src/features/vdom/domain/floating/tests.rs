use super::{
    AnchorDirection, ExclusiveZone, HorizontalAnchor, KeyboardInteractivity, PanelAnchor,
    PanelLayer, PanelSpec, PopupOffset, PopupSpec, VerticalAnchor,
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
        .with_offset(Some(PopupOffset::new(0, 10)));

    assert_eq!(spec.content(), content.as_ref());
    assert_eq!(spec.anchor_direction(), AnchorDirection::Bottom);
    assert_eq!(spec.offset(), Some(PopupOffset::new(0, 10)));
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
    let anchor = PanelAnchor::new(VerticalAnchor::Top, HorizontalAnchor::Right);
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

fn edges(anchor: PanelAnchor) -> (bool, bool, bool, bool) {
    (anchor.top(), anchor.bottom(), anchor.left(), anchor.right())
}

#[test]
fn panel_anchor_combines_vertical_and_horizontal_edges() {
    let cases = [
        (
            (VerticalAnchor::None, HorizontalAnchor::None),
            (false, false, false, false),
        ),
        (
            (VerticalAnchor::Top, HorizontalAnchor::Right),
            (true, false, false, true),
        ),
        (
            (VerticalAnchor::Bottom, HorizontalAnchor::Left),
            (false, true, true, false),
        ),
        (
            (VerticalAnchor::Both, HorizontalAnchor::Both),
            (true, true, true, true),
        ),
    ];
    for ((vertical, horizontal), expected) in cases {
        assert_eq!(edges(PanelAnchor::new(vertical, horizontal)), expected);
    }
    assert_eq!(edges(PanelAnchor::none()), (false, false, false, false));
    assert_eq!(edges(PanelAnchor::all()), (true, true, true, true));
}

#[test]
fn panel_anchor_deserializes_from_edge_flags_with_missing_edges_off() {
    let anchor: PanelAnchor = serde_json::from_str(r#"{"top": true, "right": true}"#).unwrap();
    assert_eq!(edges(anchor), (true, false, false, true));
}

#[test]
fn panel_anchor_deserializes_every_combination_of_edge_flags() {
    for top in [false, true] {
        for bottom in [false, true] {
            for left in [false, true] {
                for right in [false, true] {
                    let json = serde_json::json!({
                        "top": top, "bottom": bottom, "left": left, "right": right
                    });
                    let anchor: PanelAnchor = serde_json::from_value(json).unwrap();
                    assert_eq!(edges(anchor), (top, bottom, left, right));
                }
            }
        }
    }
}

#[test]
fn panel_anchor_deserializes_from_an_edge_or_corner_name() {
    let cases = [
        ("top", (true, false, false, false)),
        ("bottom", (false, true, false, false)),
        ("left", (false, false, true, false)),
        ("right", (false, false, false, true)),
        ("top_left", (true, false, true, false)),
        ("top-right", (true, false, false, true)),
        ("bottom-left", (false, true, true, false)),
        ("bottom_right", (false, true, false, true)),
        ("Top", (true, false, false, false)),
    ];
    for (name, expected) in cases {
        let anchor: PanelAnchor = serde_json::from_str(&format!("\"{name}\"")).unwrap();
        assert_eq!(edges(anchor), expected, "{name}");
    }
}

#[test]
fn panel_anchor_with_an_unknown_name_anchors_no_edge() {
    let anchor: PanelAnchor = serde_json::from_str("\"middle\"").unwrap();
    assert_eq!(edges(anchor), (false, false, false, false));
    let anchor: PanelAnchor = serde_json::from_str(r#"["middle", "left"]"#).unwrap();
    assert_eq!(edges(anchor), (false, false, true, false));
}

#[test]
fn panel_anchor_deserializes_from_a_list_as_the_union_of_its_edges() {
    let anchor: PanelAnchor = serde_json::from_str(r#"["top", "left", "right"]"#).unwrap();
    assert_eq!(edges(anchor), (true, false, true, true));
}

#[test]
fn panel_anchor_serializes_as_the_flags_form_it_can_read_back() {
    let anchor = PanelAnchor::new(VerticalAnchor::Bottom, HorizontalAnchor::Left);
    let json = serde_json::to_value(anchor).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"top": false, "bottom": true, "left": true, "right": false})
    );
    assert_eq!(serde_json::from_value::<PanelAnchor>(json).unwrap(), anchor);
}
