//! The attributes every element kind shares, split by what reads them, and the payloads of the
//! progress and image kinds.
use crate::features::styling::domain::{ClassNameList, ElementId, Orientation, ProgressValue};
use crate::features::vdom::domain::{ClickHandlers, UiAction, VNode};
use crate::shared::primitives::BinaryData;
use crate::shared::primitives::geometry::Size;

/// What CSS selects an element by besides its tag: its classes and its id, each absent when the
/// script set none.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ElementSelectors {
    pub(super) class: Option<ClassNameList>,
    pub(super) id: Option<ElementId>,
}

impl ElementSelectors {
    /// Selects the element by `class` and `id`, each when given.
    #[must_use]
    pub const fn new(class: Option<ClassNameList>, id: Option<ElementId>) -> Self {
        Self { class, id }
    }
}

/// How an element reacts to the pointer: the actions a click (per button) and a hover run, and
/// the tooltip shown while it is hovered, each absent when the script set none.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Interactions {
    pub(super) on_click: Option<ClickHandlers>,
    pub(super) on_hover: Option<UiAction>,
    pub(super) tooltip: Option<Box<VNode>>,
}

impl Interactions {
    /// Reacts with `on_click` and `on_hover`, and shows `tooltip`, each when given.
    #[must_use]
    pub const fn new(
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<VNode>>,
    ) -> Self {
        Self {
            on_click,
            on_hover,
            tooltip,
        }
    }

    /// The tooltip alone, for an element kind that reacts to no click or hover (an image).
    #[must_use]
    pub(crate) fn into_tooltip(self) -> Option<Box<VNode>> {
        self.tooltip
    }
}

/// What a progress bar shows: how full it is and the axis it fills along.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProgressBar {
    pub(super) value: ProgressValue,
    pub(super) orientation: Orientation,
}

impl ProgressBar {
    #[must_use]
    pub(crate) const fn new(value: ProgressValue, orientation: Orientation) -> Self {
        Self { value, orientation }
    }
}

/// An image element's pixels: the RGBA data and its size in pixels.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ImageContent {
    pub(super) data: BinaryData,
    pub(super) pixel_size: Size,
}

impl ImageContent {
    #[must_use]
    pub(crate) fn new(data: impl Into<BinaryData>, pixel_size: Size) -> Self {
        Self {
            data: data.into(),
            pixel_size,
        }
    }
}
