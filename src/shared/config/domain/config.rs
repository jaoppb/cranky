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
    #[must_use]
    pub(crate) const fn new(
        root: RootConfig,
        modules: ModulesConfig,
        rendering: RenderingMode,
        metrics: crate::features::metrics::domain::MetricsConfig,
        tooltip: TooltipConfig,
        popup: PopupConfig,
    ) -> Self {
        Self {
            root,
            modules,
            rendering,
            metrics,
            tooltip,
            popup,
        }
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
