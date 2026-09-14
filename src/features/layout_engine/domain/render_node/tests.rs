use super::*;
use crate::shared::primitives::geometry::Position;

#[test]
fn test_render_node_accessors() {
    let rect = Rect::new(Position::new(0, 0), Size::new(10, 10));
    let node = RenderNode::Rect {
        path: NodePath::root(),
        node_key: None,
        rect,
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    };
    assert_eq!(node.rect(), rect);
    assert_eq!(node.on_click(), None);
    assert_eq!(node.on_hover(), None);
    assert_eq!(node.popup(), None);
    assert_eq!(node.panel(), None);
}
