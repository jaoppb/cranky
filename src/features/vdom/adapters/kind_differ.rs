use crate::features::vdom::domain::{NodeId, Patch, VNodeKind};

#[must_use]
pub fn diff_node_kinds(
    node_id: NodeId,
    old_kind: &VNodeKind,
    new_kind: &VNodeKind,
) -> Option<Patch> {
    match (old_kind, new_kind) {
        (VNodeKind::Text { text: old_text }, VNodeKind::Text { text: new_text }) => {
            if old_text == new_text {
                Some(Patch::NoChange)
            } else {
                Some(Patch::UpdateText {
                    node_id,
                    new_text: new_text.clone(),
                })
            }
        }
        (
            VNodeKind::Progress {
                value: old_val,
                orientation: old_orient,
            },
            VNodeKind::Progress {
                value: new_val,
                orientation: new_orient,
            },
        ) => {
            if old_val == new_val && old_orient == new_orient {
                Some(Patch::NoChange)
            } else {
                Some(Patch::UpdateProgress {
                    node_id,
                    new_value: *new_val,
                    new_orientation: *new_orient,
                })
            }
        }
        (
            VNodeKind::Image {
                data: old_data,
                pixel_size: old_size,
            },
            VNodeKind::Image {
                data: new_data,
                pixel_size: new_size,
            },
        ) => {
            if old_data == new_data && old_size == new_size {
                Some(Patch::NoChange)
            } else {
                Some(Patch::UpdateImage {
                    node_id,
                    new_data: new_data.clone(),
                    new_pixel_size: *new_size,
                })
            }
        }
        (VNodeKind::Rect, VNodeKind::Rect) => Some(Patch::NoChange),
        (
            VNodeKind::Module {
                name: old_name,
                instance_id: old_instance_id,
                options: old_options,
            },
            VNodeKind::Module {
                name: new_name,
                instance_id: new_instance_id,
                options: new_options,
            },
        ) => {
            if old_name == new_name
                && old_instance_id == new_instance_id
                && old_options == new_options
            {
                Some(Patch::NoChange)
            } else {
                Some(Patch::UpdateModule {
                    node_id,
                    new_name: new_name.clone(),
                    new_instance_id: new_instance_id.clone(),
                    new_options: new_options.clone(),
                })
            }
        }
        _ => None,
    }
}
