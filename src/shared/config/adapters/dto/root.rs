use std::collections::HashMap;

use serde::Deserialize;

use crate::shared::config::domain::{self, CreatePartialRootConfigCommand, CreateRootConfigCommand};
use crate::shared::primitives::geometry::BarHeight;
use crate::shared::primitives::ModuleName;

use super::helpers::{default_height, default_root_name, json_map_to_options};
use super::margin::{MarginConfigDto, PartialMarginConfigDto};

#[derive(Debug, Deserialize)]
pub struct RootConfigDto {
    #[serde(default = "default_root_name")]
    name: String,
    #[serde(default = "default_height")]
    height: u32,
    #[serde(default)]
    vertical_alignment: VerticalAlignmentDto,
    #[serde(default)]
    margin: MarginConfigDto,
    #[serde(default)]
    unfocused: Option<PartialRootConfigDto>,
    #[serde(flatten)]
    options: HashMap<String, serde_json::Value>,
}

impl Default for RootConfigDto {
    fn default() -> Self {
        Self {
            name: default_root_name(),
            height: default_height(),
            vertical_alignment: VerticalAlignmentDto::default(),
            margin: MarginConfigDto::default(),
            unfocused: None,
            options: HashMap::new(),
        }
    }
}

impl RootConfigDto {
    #[must_use]
    pub fn into_domain(self) -> domain::RootConfig {
        domain::RootConfig::new(CreateRootConfigCommand::new(
            ModuleName::new(self.name),
            BarHeight::new(self.height),
            self.vertical_alignment.into_domain(),
            self.margin.into_domain(),
            self.unfocused.map(PartialRootConfigDto::into_domain),
            json_map_to_options(self.options),
        ))
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlignmentDto {
    Top,
    #[default]
    Center,
    Bottom,
}

impl VerticalAlignmentDto {
    #[must_use]
    pub const fn into_domain(self) -> domain::VerticalAlignment {
        match self {
            Self::Top => domain::VerticalAlignment::Top,
            Self::Center => domain::VerticalAlignment::Center,
            Self::Bottom => domain::VerticalAlignment::Bottom,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PartialRootConfigDto {
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    vertical_alignment: Option<VerticalAlignmentDto>,
    #[serde(default)]
    margin: Option<PartialMarginConfigDto>,
}

impl PartialRootConfigDto {
    #[must_use]
    pub fn into_domain(self) -> domain::PartialRootConfig {
        domain::PartialRootConfig::new(CreatePartialRootConfigCommand::new(
            self.height.map(BarHeight::new),
            self.vertical_alignment
                .map(VerticalAlignmentDto::into_domain),
            self.margin.map(PartialMarginConfigDto::into_domain),
        ))
    }
}
