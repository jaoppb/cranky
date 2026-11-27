//! The content size an unstyled leaf takes when nothing measures it: a progress bar's by its
//! orientation, and an image's.
use super::super::TaffyLayoutAdapter;
use super::support::{lay_out, root_path};
use crate::features::layout_engine::domain::StyledNode;
use crate::features::styling::domain::{ComputedStyle, Orientation, ProgressValue};
use crate::shared::primitives::BinaryData;
use crate::shared::primitives::geometry::Size;

fn progress(orientation: Orientation) -> StyledNode {
    StyledNode::Progress {
        path: root_path(),
        node_key: None,
        value: ProgressValue::new(0.5).unwrap(),
        orientation,
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

fn image() -> StyledNode {
    StyledNode::Image {
        path: root_path(),
        node_key: None,
        data: BinaryData::new(vec![0; 4]),
        pixel_size: Size::new(1, 1),
        style: ComputedStyle::default(),
        tooltip: None,
        popup: None,
        panel: None,
    }
}

#[test]
fn unstyled_progress_bars_and_images_take_their_default_size() {
    let size = |node| *lay_out(&mut TaffyLayoutAdapter::new(), node).rect().size();
    assert_eq!(size(progress(Orientation::Horizontal)), Size::new(40, 8));
    assert_eq!(size(progress(Orientation::Vertical)), Size::new(8, 40));
    assert_eq!(size(image()), Size::new(24, 24));
}
