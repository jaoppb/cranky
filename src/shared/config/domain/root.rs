use crate::shared::primitives::geometry::BarHeight;
use crate::shared::primitives::{ModuleName, ModuleOptions};

use super::margin::MarginConfig;
use super::partial_root::PartialRootConfig;
use super::types::VerticalAlignment;

#[derive(Debug, Clone, PartialEq)]
pub struct RootConfig {
    name: ModuleName,
    height: BarHeight,
    vertical_alignment: VerticalAlignment,
    margin: MarginConfig,
    unfocused: Option<PartialRootConfig>,
    options: ModuleOptions,
}

pub struct CreateRootConfigCommand {
    name: ModuleName,
    height: BarHeight,
    vertical_alignment: VerticalAlignment,
    margin: MarginConfig,
    unfocused: Option<PartialRootConfig>,
    options: ModuleOptions,
}

impl CreateRootConfigCommand {
    #[must_use]
    pub const fn new(
        name: ModuleName,
        height: BarHeight,
        vertical_alignment: VerticalAlignment,
        margin: MarginConfig,
        unfocused: Option<PartialRootConfig>,
        options: ModuleOptions,
    ) -> Self {
        Self {
            name,
            height,
            vertical_alignment,
            margin,
            unfocused,
            options,
        }
    }

    #[must_use]
    pub const fn name(&self) -> &ModuleName {
        &self.name
    }
    #[must_use]
    pub const fn height(&self) -> BarHeight {
        self.height
    }
    #[must_use]
    pub const fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }
    #[must_use]
    pub const fn margin(&self) -> &MarginConfig {
        &self.margin
    }
    #[must_use]
    pub const fn unfocused(&self) -> Option<&PartialRootConfig> {
        self.unfocused.as_ref()
    }
    #[must_use]
    pub const fn options(&self) -> &ModuleOptions {
        &self.options
    }
}

impl Default for RootConfig {
    fn default() -> Self {
        Self {
            name: ModuleName::new("bar"),
            height: BarHeight::new(30),
            vertical_alignment: VerticalAlignment::default(),
            margin: MarginConfig::default(),
            unfocused: None,
            options: ModuleOptions::default(),
        }
    }
}

impl RootConfig {
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn new(cmd: CreateRootConfigCommand) -> Self {
        Self {
            name: cmd.name().clone(),
            height: cmd.height(),
            vertical_alignment: cmd.vertical_alignment(),
            margin: *cmd.margin(),
            unfocused: cmd.unfocused().cloned(),
            options: cmd.options().clone(),
        }
    }

    #[must_use]
    pub const fn name(&self) -> &ModuleName {
        &self.name
    }

    #[must_use]
    pub const fn height(&self) -> BarHeight {
        self.height
    }

    #[cfg(test)]
    #[must_use]
    pub const fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }

    #[must_use]
    pub const fn margin(&self) -> &MarginConfig {
        &self.margin
    }

    #[must_use]
    pub const fn options(&self) -> &ModuleOptions {
        &self.options
    }

    #[must_use]
    pub fn as_unfocused(&self) -> Self {
        let mut base = self.clone();
        if let Some(unfocused) = &self.unfocused {
            if let Some(h) = unfocused.height() {
                base.height = h;
            }
            if let Some(va) = unfocused.vertical_alignment() {
                base.vertical_alignment = va;
            }
            if let Some(pm) = unfocused.margin() {
                base.margin = base.margin.with_partial_overrides(pm);
            }
        }
        base
    }
}
