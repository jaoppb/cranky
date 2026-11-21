//! Popup and panel content after styling, before layout.
use crate::features::layout_engine::domain::styled_node::StyledNode;
use crate::features::vdom::domain::{
    AnchorDirection, ExclusiveZone, KeyboardInteractivity, PanelAnchor, PanelLayer, PopupOffset,
};
use crate::shared::config::domain::MarginConfig;
use crate::shared::primitives::geometry::Rect;

/// A popup's styled content with where it opens relative to its anchor node.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledPopup {
    content: Box<StyledNode>,
    anchor_direction: AnchorDirection,
    offset: Option<PopupOffset>,
}

impl StyledPopup {
    #[must_use]
    pub(crate) const fn new(
        content: Box<StyledNode>,
        anchor_direction: AnchorDirection,
        offset: Option<PopupOffset>,
    ) -> Self {
        Self {
            content,
            anchor_direction,
            offset,
        }
    }

    #[must_use]
    pub(crate) fn content(&self) -> &StyledNode {
        &self.content
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn anchor_direction(&self) -> AnchorDirection {
        self.anchor_direction
    }

    #[must_use]
    pub(crate) const fn offset(&self) -> Option<PopupOffset> {
        self.offset
    }
}

/// A layer-shell panel's styled content with its layer, anchor edges, margin, exclusive zone
/// and keyboard mode.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledPanel {
    content: Box<StyledNode>,
    layer: PanelLayer,
    anchor: PanelAnchor,
    margin: MarginConfig,
    exclusive_zone: ExclusiveZone,
    keyboard: KeyboardInteractivity,
}

impl StyledPanel {
    #[must_use]
    pub(crate) const fn new(
        content: Box<StyledNode>,
        layer: PanelLayer,
        anchor: PanelAnchor,
        margin: MarginConfig,
        exclusive_zone: ExclusiveZone,
        keyboard: KeyboardInteractivity,
    ) -> Self {
        Self {
            content,
            layer,
            anchor,
            margin,
            exclusive_zone,
            keyboard,
        }
    }

    #[must_use]
    pub(crate) fn content(&self) -> &StyledNode {
        &self.content
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn layer(&self) -> PanelLayer {
        self.layer
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn anchor(&self) -> PanelAnchor {
        self.anchor
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn margin(&self) -> &MarginConfig {
        &self.margin
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn exclusive_zone(&self) -> ExclusiveZone {
        self.exclusive_zone
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn keyboard(&self) -> KeyboardInteractivity {
        self.keyboard
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AnchoredPopup<'a> {
    anchor_rect: Rect,
    popup: &'a StyledPopup,
}

impl<'a> AnchoredPopup<'a> {
    #[must_use]
    pub(crate) const fn new(anchor_rect: Rect, popup: &'a StyledPopup) -> Self {
        Self { anchor_rect, popup }
    }

    #[must_use]
    pub(crate) const fn anchor_rect(&self) -> &Rect {
        &self.anchor_rect
    }

    #[must_use]
    pub(crate) const fn popup(&self) -> &'a StyledPopup {
        self.popup
    }

    #[must_use]
    pub(crate) fn layout(&self) -> &'a StyledNode {
        self.popup.content()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActivePanel<'a> {
    panel: &'a StyledPanel,
}

impl<'a> ActivePanel<'a> {
    #[must_use]
    pub(crate) const fn new(panel: &'a StyledPanel) -> Self {
        Self { panel }
    }

    #[must_use]
    pub(crate) fn layout(&self) -> &'a StyledNode {
        self.panel.content()
    }
}
