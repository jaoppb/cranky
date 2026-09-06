use super::module::ModulesConfig;
use super::root::RootConfig;
use super::style::{PopupConfig, TooltipConfig};
use super::types::RenderingMode;

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
    pub const fn new(
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

    #[must_use]
    pub const fn root(&self) -> &RootConfig {
        &self.root
    }

    #[must_use]
    pub const fn modules(&self) -> &ModulesConfig {
        &self.modules
    }

    #[must_use]
    pub const fn metrics(&self) -> &crate::features::metrics::domain::MetricsConfig {
        &self.metrics
    }

    #[must_use]
    pub const fn tooltip(&self) -> &TooltipConfig {
        &self.tooltip
    }

    #[must_use]
    pub const fn popup(&self) -> &PopupConfig {
        &self.popup
    }
}
