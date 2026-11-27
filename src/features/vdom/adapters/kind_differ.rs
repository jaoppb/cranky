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

/// The patch that gives `node_id` what a changed leaf `new_kind` holds: a module's parameters
/// here, a text, progress or image content through `content_update`.
fn kind_update(node_id: NodeId, new_kind: &VNodeKind) -> Patch {
    if let VNodeKind::Module {
        name,
        instance_id,
        options,
    } = new_kind
    {
        return Patch::UpdateModule {
            node_id,
            new_name: name.clone(),
            new_instance_id: instance_id.clone(),
            new_options: options.clone(),
        };
    }
    content_update(node_id, new_kind)
}

/// The patch that gives `node_id` the content of a text, progress or image `new_kind`. The
/// fallback arm is never reached: a module is handled by `kind_update`, containers are filtered
/// out by `same_leaf_kind`, and a rect holds nothing, so two rects are always equal and
/// `diff_node_kinds` returns `NoChange` before asking for an update.
fn content_update(node_id: NodeId, new_kind: &VNodeKind) -> Patch {
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
        _ => Patch::NoChange,
    }
}
