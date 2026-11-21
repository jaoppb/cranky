//! Settings of the root module, the bar itself: its script, height, alignment, margins and options.
use crate::shared::primitives::geometry::BarHeight;
use crate::shared::primitives::{ModuleName, ModuleOptions};

use super::margin::MarginConfig;
use super::partial_root::PartialRootConfig;
use super::types::VerticalAlignment;

/// Settings of the root module, the bar itself, from the `[root]` table.
/// `[root.unfocused]` overrides height and margin on unfocused monitors (its alignment is parsed
/// but unused).
#[derive(Debug, Clone, PartialEq)]
pub struct RootConfig {
    name: ModuleName,
    height: BarHeight,
    vertical_alignment: VerticalAlignment,
    margin: MarginConfig,
    unfocused: Option<PartialRootConfig>,
    options: ModuleOptions,
}

pub(crate) struct CreateRootConfigCommand {
    name: ModuleName,
    height: BarHeight,
    vertical_alignment: VerticalAlignment,
    margin: MarginConfig,
    unfocused: Option<PartialRootConfig>,
    options: ModuleOptions,
}

impl CreateRootConfigCommand {
    #[must_use]
    pub(crate) const fn new(
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
    #[must_use]
    pub(crate) fn new(cmd: CreateRootConfigCommand) -> Self {
        Self {
            name: cmd.name,
            height: cmd.height,
            vertical_alignment: cmd.vertical_alignment,
            margin: cmd.margin,
            unfocused: cmd.unfocused,
            options: cmd.options,
        }
    }

    #[must_use]
    pub(crate) const fn name(&self) -> &ModuleName {
        &self.name
    }

    /// Height the bar's layer surface is sized to, in logical pixels.
    #[must_use]
    pub const fn height(&self) -> BarHeight {
        self.height
    }

    /// Vertical alignment set by `vertical_alignment` in `[root]`.
    #[cfg(test)]
    #[must_use]
    pub const fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }

    /// Margins applied to the bar's layer surface.
    #[must_use]
    pub const fn margin(&self) -> &MarginConfig {
        &self.margin
    }

    /// The remaining `[root]` keys: their `left`/`center`/`right` arrays pick the slot modules, and
    /// the whole table is handed to the bar script as its options when `[modules]` has no entry for
    /// the root.
    #[must_use]
    pub const fn options(&self) -> &ModuleOptions {
        &self.options
    }

    #[must_use]
    pub(crate) fn as_unfocused(&self) -> Self {
        let mut base = self.clone();
        if let Some(unfocused) = &self.unfocused {
            if let Some(height) = unfocused.height() {
                base.height = height;
            }
            if let Some(vertical_alignment) = unfocused.vertical_alignment() {
                base.vertical_alignment = vertical_alignment;
            }
            if let Some(margin) = unfocused.margin() {
                base.margin = base.margin.with_partial_overrides(margin);
            }
        }
        base
    }
}
