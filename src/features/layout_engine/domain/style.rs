//! Flexbox alignment enums and box spacing used by the computed style.
use serde::{Deserialize, Serialize};

/// Main axis of a flex container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FlexDirection {
    /// Children run left to right.
    #[default]
    Row,
    /// Children run top to bottom.
    Column,
}

/// How a flex container distributes free space along its main axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum JustifyContent {
    /// Children packed at the start of the axis.
    #[default]
    Start,
    /// Children packed at the end of the axis.
    End,
    /// Children packed in the middle of the axis.
    Center,
    /// Free space split between children, none at the edges.
    SpaceBetween,
    /// Free space split around each child, so the edges get half a gap.
    SpaceAround,
    /// Free space split into equal gaps, the edges included.
    SpaceEvenly,
}

/// How a flex container places children on its cross axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlignItems {
    /// Children aligned to the start of the cross axis.
    #[default]
    Start,
    /// Children aligned to the end of the cross axis.
    End,
    /// Children centered on the cross axis.
    Center,
    /// Children stretched to fill the cross axis.
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PositionType {
    #[default]
    Relative,
    Absolute,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct Gap {
    value: f64,
}

impl Gap {
    #[must_use]
    pub(crate) const fn new(value: f64) -> Self {
        Self { value }
    }
    #[must_use]
    pub(crate) const fn value(&self) -> f64 {
        self.value
    }
}

/// Per-side lengths in logical pixels, used for both CSS `padding` and `margin`.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct BoxMargin {
    #[serde(default)]
    top: f64,
    #[serde(default)]
    bottom: f64,
    #[serde(default)]
    left: f64,
    #[serde(default)]
    right: f64,
}

impl BoxMargin {
    /// Takes the sides as top, bottom, left, right, not in the clockwise CSS shorthand order.
    #[must_use]
    pub const fn new(top: f64, bottom: f64, left: f64, right: f64) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }

    #[must_use]
    pub(crate) const fn top(&self) -> f64 {
        self.top
    }
    #[must_use]
    pub(crate) const fn bottom(&self) -> f64 {
        self.bottom
    }
    #[must_use]
    pub(crate) const fn left(&self) -> f64 {
        self.left
    }
    #[must_use]
    pub(crate) const fn right(&self) -> f64 {
        self.right
    }
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub(crate) struct FlexStyle {
    #[serde(default)]
    direction: FlexDirection,
    #[serde(default)]
    justify: JustifyContent,
    #[serde(default)]
    align_items: AlignItems,
    #[serde(default)]
    padding: BoxMargin,
    #[serde(default)]
    margin: BoxMargin,
    #[serde(default)]
    gap: Option<Gap>,
    #[serde(default)]
    position: PositionType,
}

#[cfg(test)]
impl FlexStyle {
    #[must_use]
    pub(crate) const fn direction(&self) -> FlexDirection {
        self.direction
    }
    #[must_use]
    pub(crate) const fn justify(&self) -> JustifyContent {
        self.justify
    }
    #[must_use]
    pub(crate) const fn align_items(&self) -> AlignItems {
        self.align_items
    }
    #[must_use]
    pub(crate) const fn padding(&self) -> &BoxMargin {
        &self.padding
    }
    #[must_use]
    pub(crate) const fn margin(&self) -> &BoxMargin {
        &self.margin
    }
    #[must_use]
    pub(crate) const fn gap(&self) -> Option<&Gap> {
        self.gap.as_ref()
    }
    #[must_use]
    pub(crate) const fn position(&self) -> PositionType {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_margin() {
        let margin = BoxMargin {
            top: 1.0,
            bottom: 2.0,
            left: 3.0,
            right: 4.0,
        };
        assert!((margin.top() - 1.0).abs() < f64::EPSILON);
        assert!((margin.bottom() - 2.0).abs() < f64::EPSILON);
        assert!((margin.left() - 3.0).abs() < f64::EPSILON);
        assert!((margin.right() - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_flex_style() {
        let style = FlexStyle {
            direction: FlexDirection::Column,
            justify: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            position: PositionType::Absolute,
            padding: BoxMargin {
                top: 1.0,
                bottom: 1.0,
                left: 1.0,
                right: 1.0,
            },
            margin: BoxMargin::default(),
            gap: Some(Gap { value: 5.0 }),
        };

        assert_eq!(style.direction(), FlexDirection::Column);
        assert_eq!(style.justify(), JustifyContent::Center);
        assert_eq!(style.align_items(), AlignItems::Stretch);
        assert_eq!(style.position(), PositionType::Absolute);
        assert!((style.padding().top() - 1.0).abs() < f64::EPSILON);
        assert!((style.margin().top() - 0.0).abs() < f64::EPSILON);
        assert!((style.gap().unwrap().value() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_box_margin_new_takes_top_bottom_left_right() {
        let margin = BoxMargin::new(1.5, 2.5, 3.5, 4.5);
        assert_eq!(
            margin,
            BoxMargin {
                top: 1.5,
                bottom: 2.5,
                left: 3.5,
                right: 4.5,
            }
        );
    }
}
