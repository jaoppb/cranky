use super::anchor::PanelAnchor;
use super::types::{ExclusiveZone, KeyboardInteractivity, PanelLayer};
use crate::features::vdom::domain::VNode;
use crate::shared::config::domain::MarginConfig;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PanelSpec {
    content: Box<VNode>,
    #[serde(default)]
    layer: PanelLayer,
    #[serde(default)]
    anchor: PanelAnchor,
    #[serde(default)]
    margin: MarginConfig,
    #[serde(default)]
    exclusive_zone: ExclusiveZone,
    #[serde(default)]
    keyboard: KeyboardInteractivity,
}

impl PanelSpec {
    #[must_use]
    pub fn new(content: Box<VNode>) -> Self {
        Self {
            content,
            layer: PanelLayer::Top,
            anchor: PanelAnchor::none(),
            margin: MarginConfig::default(),
            exclusive_zone: ExclusiveZone::none(),
            keyboard: KeyboardInteractivity::None,
        }
    }

    #[must_use]
    pub const fn with_layer(mut self, layer: PanelLayer) -> Self {
        self.layer = layer;
        self
    }

    #[must_use]
    pub const fn with_anchor(mut self, anchor: PanelAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    #[must_use]
    pub const fn with_margin(mut self, margin: MarginConfig) -> Self {
        self.margin = margin;
        self
    }

    #[must_use]
    pub const fn with_exclusive_zone(mut self, exclusive_zone: ExclusiveZone) -> Self {
        self.exclusive_zone = exclusive_zone;
        self
    }

    #[must_use]
    pub const fn with_keyboard(mut self, keyboard: KeyboardInteractivity) -> Self {
        self.keyboard = keyboard;
        self
    }

    #[must_use]
    pub fn content(&self) -> &VNode {
        &self.content
    }

    #[must_use]
    pub const fn layer(&self) -> PanelLayer {
        self.layer
    }

    #[must_use]
    pub const fn anchor(&self) -> PanelAnchor {
        self.anchor
    }

    #[must_use]
    pub const fn margin(&self) -> &MarginConfig {
        &self.margin
    }

    #[must_use]
    pub const fn exclusive_zone(&self) -> ExclusiveZone {
        self.exclusive_zone
    }

    #[must_use]
    pub const fn keyboard(&self) -> KeyboardInteractivity {
        self.keyboard
    }
}
