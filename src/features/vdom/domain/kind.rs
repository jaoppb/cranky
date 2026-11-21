//! `VNodeKind`: what a node is (flex, grid, text, progress, rect, image or embedded module) and the
//! data specific to that kind.
use super::tag::TextContent;
use super::vnode::VNode;
use crate::features::styling::domain::{Orientation, ProgressValue};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{BinaryData, ModuleInstanceId, ModuleName, ModuleOptions};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum VNodeKind {
    #[serde(rename = "flex")]
    Flex {
        #[serde(default)]
        children: Vec<VNode>,
    },
    #[serde(rename = "grid")]
    Grid {
        #[serde(default)]
        children: Vec<VNode>,
    },
    #[serde(rename = "text")]
    Text { text: TextContent },
    #[serde(rename = "progress")]
    Progress {
        #[serde(default)]
        value: ProgressValue,
        #[serde(default)]
        orientation: Orientation,
    },
    #[serde(rename = "rect")]
    Rect,
    #[serde(rename = "image")]
    Image { data: BinaryData, pixel_size: Size },
    #[serde(rename = "module")]
    Module {
        name: ModuleName,
        #[serde(default)]
        instance_id: Option<ModuleInstanceId>,
        #[serde(default)]
        options: ModuleOptions,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModuleParams {
    name: ModuleName,
    instance_id: Option<ModuleInstanceId>,
    options: ModuleOptions,
}

impl ModuleParams {
    #[must_use]
    pub(crate) const fn new(
        name: ModuleName,
        instance_id: Option<ModuleInstanceId>,
        options: ModuleOptions,
    ) -> Self {
        Self {
            name,
            instance_id,
            options,
        }
    }

    #[must_use]
    pub(crate) fn into_parts(self) -> (ModuleName, Option<ModuleInstanceId>, ModuleOptions) {
        (self.name, self.instance_id, self.options)
    }
}
