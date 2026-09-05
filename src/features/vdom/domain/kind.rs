use super::tag::TextContent;
use super::vnode::VNode;
use crate::features::styling::domain::{Orientation, ProgressValue};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{
    BinaryData, ModuleInstanceId, ModuleName, ModuleOptions,
};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum VNodeKind {
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
