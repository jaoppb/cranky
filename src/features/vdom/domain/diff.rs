use super::identifier::NodeId;
use super::tag::TextContent;
use super::vnode::VNode;
use crate::features::styling::domain::{Orientation, ProgressValue};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{BinaryData, ModuleInstanceId, ModuleName, ModuleOptions};

#[derive(Debug, Clone, PartialEq)]
pub enum Patch {
    NoChange,
    Replace {
        old_node_id: NodeId,
        new_node: Box<VNode>,
    },
    UpdateProps {
        node_id: NodeId,
        class_changed: bool,
        id_changed: bool,
        handlers_changed: bool,
        tooltip_patch: Option<Box<Self>>,
        popup_patch: Option<Box<Self>>,
        kind_patch: Box<Self>,
    },
    UpdateText {
        node_id: NodeId,
        new_text: TextContent,
    },
    UpdateProgress {
        node_id: NodeId,
        new_value: ProgressValue,
        new_orientation: Orientation,
    },
    UpdateImage {
        node_id: NodeId,
        new_data: BinaryData,
        new_pixel_size: Size,
    },
    UpdateModule {
        node_id: NodeId,
        new_name: ModuleName,
        new_instance_id: Option<ModuleInstanceId>,
        new_options: ModuleOptions,
    },
    UpdateChildren {
        node_id: NodeId,
        child_patches: Vec<ChildPatchOp>,
    },
}

impl Patch {
    #[must_use]
    pub const fn is_no_change(&self) -> bool {
        matches!(self, Self::NoChange)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChildPatchOp {
    Insert {
        index: usize,
        node: Box<VNode>,
    },
    Remove {
        node_id: NodeId,
        index: usize,
    },
    Move {
        node_id: NodeId,
        from: usize,
        to: usize,
    },
    Update {
        node_id: NodeId,
        patch: Box<Patch>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffResult {
    patch: Patch,
}

impl DiffResult {
    #[must_use]
    pub const fn new(patch: Patch) -> Self {
        Self { patch }
    }

    #[must_use]
    pub const fn unchanged() -> Self {
        Self {
            patch: Patch::NoChange,
        }
    }

    #[must_use]
    pub const fn is_unchanged(&self) -> bool {
        self.patch.is_no_change()
    }

    #[must_use]
    pub const fn patch(&self) -> &Patch {
        &self.patch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_result() {
        let res = DiffResult::unchanged();
        assert!(res.is_unchanged());
        assert_eq!(res.patch(), &Patch::NoChange);
    }
}
