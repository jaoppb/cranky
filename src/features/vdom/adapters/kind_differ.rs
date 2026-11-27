//! Diffing of what two nodes of the same tag hold besides children: text, progress, image and
//! embedded-module parameters.
use crate::features::vdom::domain::{NodeId, Patch, VNodeKind};
use std::mem::discriminant;

#[must_use]
pub(crate) fn diff_node_kinds(
    node_id: NodeId,
    old_kind: &VNodeKind,
    new_kind: &VNodeKind,
) -> Option<Patch> {
    if !same_leaf_kind(old_kind, new_kind) {
        return None;
    }
    if old_kind == new_kind {
        Some(Patch::NoChange)
    } else {
        Some(kind_update(node_id, new_kind))
    }
}

/// Whether both kinds are the same leaf kind, the only pairs this differ patches; containers
/// and mismatched kinds are left to the caller.
fn same_leaf_kind(old_kind: &VNodeKind, new_kind: &VNodeKind) -> bool {
    discriminant(old_kind) == discriminant(new_kind)
        && !matches!(new_kind, VNodeKind::Flex { .. } | VNodeKind::Grid { .. })
}

/// The patch that gives `node_id` the content of a text, progress or image `new_kind`; the
/// other kinds are left to `module_update`.
fn kind_update(node_id: NodeId, new_kind: &VNodeKind) -> Patch {
    match new_kind {
        VNodeKind::Text { text } => Patch::UpdateText {
            node_id,
            new_text: text.clone(),
        },
        VNodeKind::Progress { value, orientation } => Patch::UpdateProgress {
            node_id,
            new_value: *value,
            new_orientation: *orientation,
        },
        VNodeKind::Image { data, pixel_size } => Patch::UpdateImage {
            node_id,
            new_data: data.clone(),
            new_pixel_size: *pixel_size,
        },
        _ => module_update(node_id, new_kind),
    }
}

/// The patch that gives `node_id` the parameters of a module `new_kind`; a rect has no content
/// to update, and containers never reach here.
fn module_update(node_id: NodeId, new_kind: &VNodeKind) -> Patch {
    match new_kind {
        VNodeKind::Module {
            name,
            instance_id,
            options,
        } => Patch::UpdateModule {
            node_id,
            new_name: name.clone(),
            new_instance_id: instance_id.clone(),
            new_options: options.clone(),
        },
        _ => Patch::NoChange,
    }
}
