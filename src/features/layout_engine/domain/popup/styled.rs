use crate::features::layout_engine::domain::styled_node::StyledNode;
use crate::features::vdom::domain::{
    AnchorDirection, ExclusiveZone, KeyboardInteractivity, PanelAnchor, PanelLayer, PopupOffset,
};
use crate::shared::config::domain::MarginConfig;
use crate::shared::primitives::geometry::Rect;

#[derive(Debug, Clone, PartialEq)]
pub struct StyledPopup {
    content: Box<StyledNode>,
    anchor_direction: AnchorDirection,
    offset: Option<PopupOffset>,
    dismiss_on_unfocus: bool,
}

impl StyledPopup {
    #[must_use]
    pub const fn new(
        content: Box<StyledNode>,
        anchor_direction: AnchorDirection,
        offset: Option<PopupOffset>,
        dismiss_on_unfocus: bool,
    ) -> Self {
        Self {
            content,
            anchor_direction,
            offset,
            dismiss_on_unfocus,
        }
    }

    #[must_use]
    pub fn content(&self) -> &StyledNode {
        &self.content
    }

    #[must_use]
    pub const fn anchor_direction(&self) -> AnchorDirection {
        self.anchor_direction
    }

    #[must_use]
    pub const fn offset(&self) -> Option<PopupOffset> {
        self.offset
    }

    #[must_use]
    pub const fn dismiss_on_unfocus(&self) -> bool {
        self.dismiss_on_unfocus
    }
}

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
    pub const fn new(
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
    pub fn content(&self) -> &StyledNode {
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

#[derive(Debug, Clone, PartialEq)]
pub struct AnchoredPopup<'a> {
    anchor_rect: Rect,
    popup: &'a StyledPopup,
}

impl<'a> AnchoredPopup<'a> {
    #[must_use]
    pub const fn new(anchor_rect: Rect, popup: &'a StyledPopup) -> Self {
        Self { anchor_rect, popup }
    }

    #[must_use]
    pub const fn anchor_rect(&self) -> &Rect {
        &self.anchor_rect
    }

    #[must_use]
    pub const fn popup(&self) -> &'a StyledPopup {
        self.popup
    }

    #[must_use]
    pub fn layout(&self) -> &'a StyledNode {
        self.popup.content()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivePanel<'a> {
    panel: &'a StyledPanel,
}

impl<'a> ActivePanel<'a> {
    #[must_use]
    pub const fn new(panel: &'a StyledPanel) -> Self {
        Self { panel }
    }

    #[must_use]
    pub const fn panel(&self) -> &'a StyledPanel {
        self.panel
    }

    #[must_use]
    pub fn layout(&self) -> &'a StyledNode {
        self.panel.content()
    }
}
