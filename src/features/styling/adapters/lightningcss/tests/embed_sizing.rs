use crate::features::layout_engine::domain::LayoutRoot;
use crate::features::layout_engine::domain::TextMeasurer;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::{ClassNameList, CssLength, StyleSheetName};
use crate::features::styling::ports::CssParserPort;
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
use crate::shared::primitives::SizeConstraint;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::rendering::ports::text::TextFont;

/// A measurer whose embedded child reports the calendar's own 240x190 default and whose text
/// takes no room.
struct CalendarMeasurer;
impl TextMeasurer for CalendarMeasurer {
    fn measure(&mut self, _t: &str, _font: TextFont<'_>) -> Size {
        Size::new(0, 0)
    }
    fn measure_module(&self, _key: &crate::shared::primitives::ModuleKey) -> Option<Size> {
        Some(Size::new(240, 190))
    }
}

/// A resolver over the shipped `base.css` and `clock.css`, in that order.
fn shipped_clock_resolver() -> CompositeStyleResolver {
    let parser = LightningCssAdapter::new();
    let sheets = [
        (
            "base",
            include_str!("../../../../../../assets/styles/base.css"),
        ),
        (
            "clock",
            include_str!("../../../../../../assets/styles/clock.css"),
        ),
    ]
    .into_iter()
    .map(|(name, css)| {
        parser
            .parse_stylesheet(StyleSheetName::new(name).unwrap(), css)
            .unwrap()
    })
    .collect();
    CompositeStyleResolver::new(sheets)
}

/// The module node the clock popup embeds the calendar through, with the `calendar` class.
fn calendar_embed() -> VNode {
    VNode::new_module(
        crate::features::vdom::domain::ModuleParams::new(
            crate::shared::primitives::ModuleName::new("calendar".to_string()),
            None,
            crate::shared::primitives::ModuleOptions::default(),
        ),
        ElementSelectors::new(Some(ClassNameList::parse("calendar").unwrap()), None),
        Interactions::default(),
    )
}

#[test]
fn test_shipped_clock_css_gives_the_calendar_embed_a_280_wide_host() {
    // Issue #9: the popup measured ~240 wide even though `clock.css` sets
    // `.calendar { min-width: 280px }` on the embed node. Resolve the real
    // shipped sheets against a real Module node and lay it out with the
    // child reporting its own 240 default.
    let resolver = shipped_clock_resolver();
    let styled = calendar_embed().resolve_styles(&resolver, None);
    assert_eq!(styled.style().min_width(), Some(CssLength::Pixels(280.0)));

    let mut engine = crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter::new();
    let node = engine
        .calculate_layout(
            LayoutRoot::new(styled, Position::new(0, 0), SizeConstraint::none()),
            &mut CalendarMeasurer,
        )
        .unwrap();
    assert_eq!(node.rect().width(), 280);
}
