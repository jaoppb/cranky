use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum JustifyContent {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlignItems {
    #[default]
    Start,
    End,
    Center,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PositionType {
    #[default]
    Relative,
    Absolute,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Gap {
    value: f64,
}

impl Gap {
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self { value }
    }
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.value
    }
}

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
    pub const fn top(&self) -> f64 {
        self.top
    }
    #[must_use]
    pub const fn bottom(&self) -> f64 {
        self.bottom
    }
    #[must_use]
    pub const fn left(&self) -> f64 {
        self.left
    }
    #[must_use]
    pub const fn right(&self) -> f64 {
        self.right
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct FlexStyle {
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

impl FlexStyle {
    #[must_use]
    pub const fn with_padding(mut self, padding: BoxMargin) -> Self {
        self.padding = padding;
        self
    }
    #[must_use]
    pub const fn direction(&self) -> FlexDirection {
        self.direction
    }
    #[must_use]
    pub const fn justify(&self) -> JustifyContent {
        self.justify
    }
    #[must_use]
    pub const fn align_items(&self) -> AlignItems {
        self.align_items
    }
    #[must_use]
    pub const fn padding(&self) -> &BoxMargin {
        &self.padding
    }
    #[must_use]
    pub const fn margin(&self) -> &BoxMargin {
        &self.margin
    }
    #[must_use]
    pub const fn gap(&self) -> Option<&Gap> {
        self.gap.as_ref()
    }
    #[must_use]
    pub const fn position(&self) -> PositionType {
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
}
