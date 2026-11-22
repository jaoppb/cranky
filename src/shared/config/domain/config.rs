//! The whole parsed configuration, as published on the hub's config channel.
use super::module::ModulesConfig;
use super::root::RootConfig;
use super::style::{PopupConfig, TooltipConfig};
use super::types::RenderingMode;

/// Everything `config.toml` configures, published on the signal hub's config channel and re-sent
/// on every edit that parses.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    root: RootConfig,
    modules: ModulesConfig,
    rendering: RenderingMode,
    metrics: crate::features::metrics::domain::MetricsConfig,
    tooltip: TooltipConfig,
    popup: PopupConfig,
}

impl Config {
    /// This configuration with its `[root]` table replaced; every table of `config.toml` is
    /// optional and defaults on its own, so each one is set on its own.
    #[must_use]
    pub(crate) fn with_root(mut self, root: RootConfig) -> Self {
        self.root = root;
        self
    }

    /// This configuration with its `[modules]` table replaced.
    #[must_use]
    pub(crate) fn with_modules(mut self, modules: ModulesConfig) -> Self {
        self.modules = modules;
        self
    }

    /// This configuration with its `[rendering]` table replaced.
    #[must_use]
    pub(crate) const fn with_rendering(mut self, rendering: RenderingMode) -> Self {
        self.rendering = rendering;
        self
    }

    /// This configuration with its `[metrics]` table replaced.
    #[must_use]
    pub(crate) const fn with_metrics(
        mut self,
        metrics: crate::features::metrics::domain::MetricsConfig,
    ) -> Self {
        self.metrics = metrics;
        self
    }

    /// This configuration with its `[tooltip]` table replaced.
    #[must_use]
    pub(crate) fn with_tooltip(mut self, tooltip: TooltipConfig) -> Self {
        self.tooltip = tooltip;
        self
    }

    /// This configuration with its `[popup]` table replaced.
    #[must_use]
    pub(crate) const fn with_popup(mut self, popup: PopupConfig) -> Self {
        self.popup = popup;
        self
    }

    /// Settings of the bar itself, from the `[root]` table.
    #[must_use]
    pub const fn root(&self) -> &RootConfig {
        &self.root
    }

    /// Per-module settings, from the `[modules]` table.
    #[must_use]
    pub const fn modules(&self) -> &ModulesConfig {
        &self.modules
    }

    /// Settings of the system metrics sampler, from the `[metrics]` table.
    #[must_use]
    pub const fn metrics(&self) -> &crate::features::metrics::domain::MetricsConfig {
        &self.metrics
    }

    /// Tooltip settings, from the `[tooltip]` table.
    #[must_use]
    pub const fn tooltip(&self) -> &TooltipConfig {
        &self.tooltip
    }

    /// How popups are grouped, offset and embedded, from the `[popup]` table.
    #[must_use]
    pub const fn popup(&self) -> &PopupConfig {
        &self.popup
    }
}
