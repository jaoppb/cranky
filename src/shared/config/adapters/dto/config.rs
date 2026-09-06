use serde::Deserialize;

use crate::shared::config::domain;
use crate::shared::rendering::ports::font::FontValidatorPort;

use super::module::ModulesConfigDto;
use super::root::RootConfigDto;
use super::style::{PopupConfigDto, RenderingModeDto, TooltipConfigDto};

#[derive(Debug, Deserialize, Default)]
pub struct ConfigDto {
    #[serde(default)]
    root: RootConfigDto,
    #[serde(default)]
    modules: ModulesConfigDto,
    #[serde(default)]
    rendering: RenderingModeDto,
    #[serde(default)]
    metrics: crate::features::metrics::domain::MetricsConfig,
    #[serde(default)]
    tooltip: TooltipConfigDto,
    #[serde(default)]
    popup: PopupConfigDto,
}

impl ConfigDto {
    #[must_use]
    pub fn into_domain<V: FontValidatorPort>(self, _validator: &V) -> domain::Config {
        let root = self.root.into_domain();
        let modules = self.modules.into_domain();
        let rendering = self.rendering.into_domain();
        let tooltip = self.tooltip.into_domain();
        let popup = self.popup.into_domain();

        domain::Config::new(root, modules, rendering, self.metrics, tooltip, popup)
    }
}
