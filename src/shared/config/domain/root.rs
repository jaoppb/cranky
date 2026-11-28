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
    /// These settings with the bar's script name replaced; every `[root]` key is optional and
    /// defaults on its own, so each one is set on its own.
    #[must_use]
    pub(crate) fn with_name(mut self, name: ModuleName) -> Self {
        self.name = name;
        self
    }

    /// These settings with the bar height replaced.
    #[must_use]
    pub(crate) const fn with_height(mut self, height: BarHeight) -> Self {
        self.height = height;
        self
    }

    /// These settings with the vertical alignment replaced.
    #[must_use]
    pub(crate) const fn with_vertical_alignment(
        mut self,
        vertical_alignment: VerticalAlignment,
    ) -> Self {
        self.vertical_alignment = vertical_alignment;
        self
    }

    /// These settings with the margins replaced.
    #[must_use]
    pub(crate) const fn with_margin(mut self, margin: MarginConfig) -> Self {
        self.margin = margin;
        self
    }

    /// These settings with the `[root.unfocused]` overrides replaced.
    #[must_use]
    pub(crate) const fn with_unfocused(mut self, unfocused: Option<PartialRootConfig>) -> Self {
        self.unfocused = unfocused;
        self
    }

    /// These settings with the remaining `[root]` keys replaced.
    #[must_use]
    pub(crate) fn with_options(mut self, options: ModuleOptions) -> Self {
        self.options = options;
        self
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
        let Some(unfocused) = &self.unfocused else {
            return base;
        };
        if let Some(height) = unfocused.height() {
            base.height = height;
        }
        if let Some(vertical_alignment) = unfocused.vertical_alignment() {
            base.vertical_alignment = vertical_alignment;
        }
        if let Some(margin) = unfocused.margin() {
            base.margin = base.margin.with_partial_overrides(margin);
        }
        base
    }
}
