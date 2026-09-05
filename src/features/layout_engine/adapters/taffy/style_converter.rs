use crate::features::layout_engine::domain::{
    AlignItems, BoxMargin, FlexDirection, Gap, JustifyContent, PositionType,
};
use crate::features::styling::domain::CssLength;
use taffy::geometry::Size as TaffySize;
use taffy::style::Dimension;
use taffy::style::LengthPercentage;
use taffy::style::{
    AlignItems as TaffyAlignItems, FlexDirection as TaffyFlexDirection,
    JustifyContent as TaffyJustifyContent,
};

impl From<FlexDirection> for TaffyFlexDirection {
    fn from(dir: FlexDirection) -> Self {
        match dir {
            FlexDirection::Row => Self::Row,
            FlexDirection::Column => Self::Column,
        }
    }
}

impl From<JustifyContent> for TaffyJustifyContent {
    fn from(jc: JustifyContent) -> Self {
        match jc {
            JustifyContent::Start => Self::FLEX_START,
            JustifyContent::End => Self::FLEX_END,
            JustifyContent::Center => Self::CENTER,
            JustifyContent::SpaceBetween => Self::SPACE_BETWEEN,
            JustifyContent::SpaceAround => Self::SPACE_AROUND,
            JustifyContent::SpaceEvenly => Self::SPACE_EVENLY,
        }
    }
}

impl From<AlignItems> for TaffyAlignItems {
    fn from(ai: AlignItems) -> Self {
        match ai {
            AlignItems::Start => Self::FLEX_START,
            AlignItems::End => Self::FLEX_END,
            AlignItems::Center => Self::CENTER,
            AlignItems::Stretch => Self::STRETCH,
        }
    }
}

impl From<PositionType> for taffy::style::Position {
    fn from(pt: PositionType) -> Self {
        match pt {
            PositionType::Relative => Self::Relative,
            PositionType::Absolute => Self::Absolute,
        }
    }
}

#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
impl From<&BoxMargin> for taffy::geometry::Rect<LengthPercentage> {
    fn from(padding: &BoxMargin) -> Self {
        Self {
            left: LengthPercentage::length(padding.left() as f32),
            right: LengthPercentage::length(padding.right() as f32),
            top: LengthPercentage::length(padding.top() as f32),
            bottom: LengthPercentage::length(padding.bottom() as f32),
        }
    }
}

#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
impl From<&BoxMargin> for taffy::geometry::Rect<taffy::style::LengthPercentageAuto> {
    fn from(margin: &BoxMargin) -> Self {
        Self {
            left: LengthPercentage::length(margin.left() as f32).into(),
            right: LengthPercentage::length(margin.right() as f32).into(),
            top: LengthPercentage::length(margin.top() as f32).into(),
            bottom: LengthPercentage::length(margin.bottom() as f32).into(),
        }
    }
}

#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
impl From<&Gap> for TaffySize<LengthPercentage> {
    fn from(gap: &Gap) -> Self {
        Self {
            width: LengthPercentage::length(gap.value() as f32),
            height: LengthPercentage::length(gap.value() as f32),
        }
    }
}

impl From<CssLength> for Dimension {
    fn from(l: CssLength) -> Self {
        match l {
            CssLength::Px(v) => Self::length(v),
            CssLength::Percent(v) => Self::percent(v / 100.0),
            CssLength::Auto => Self::auto(),
        }
    }
}
