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

use crate::utils::f64_to_f32;

impl From<&BoxMargin> for taffy::geometry::Rect<LengthPercentage> {
    fn from(padding: &BoxMargin) -> Self {
        Self {
            left: LengthPercentage::length(f64_to_f32(padding.left())),
            right: LengthPercentage::length(f64_to_f32(padding.right())),
            top: LengthPercentage::length(f64_to_f32(padding.top())),
            bottom: LengthPercentage::length(f64_to_f32(padding.bottom())),
        }
    }
}

impl From<&BoxMargin> for taffy::geometry::Rect<taffy::style::LengthPercentageAuto> {
    fn from(margin: &BoxMargin) -> Self {
        Self {
            left: LengthPercentage::length(f64_to_f32(margin.left())).into(),
            right: LengthPercentage::length(f64_to_f32(margin.right())).into(),
            top: LengthPercentage::length(f64_to_f32(margin.top())).into(),
            bottom: LengthPercentage::length(f64_to_f32(margin.bottom())).into(),
        }
    }
}

impl From<&Gap> for TaffySize<LengthPercentage> {
    fn from(gap: &Gap) -> Self {
        Self {
            width: LengthPercentage::length(f64_to_f32(gap.value())),
            height: LengthPercentage::length(f64_to_f32(gap.value())),
        }
    }
}

impl From<CssLength> for Dimension {
    fn from(l: CssLength) -> Self {
        match l {
            CssLength::Px(v) => Self::length(v),
            CssLength::Percent(v) => Self::percent(v / 100.0),
            CssLength::Auto => Self::auto(),
            // First-pass placeholder: taffy has no native calc() hook this
            // crate can use without unsafe code (see mod.rs), so a `Calc`
            // dimension starts as just its absolute part. `mod.rs`'s second
            // layout pass overwrites this once the containing block's size
            // — needed to resolve `percent` — is known.
            CssLength::Calc { px, .. } => Self::length(px),
        }
    }
}
