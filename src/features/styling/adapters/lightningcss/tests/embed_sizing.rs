use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::layout_engine::domain::TextMeasurer;
use crate::features::styling::domain::{ClassNameList, CssLength, StyleSheetName};
use crate::features::styling::ports::CssParserPort;
use crate::features::vdom::domain::VNode;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{Position, Size};

#[test]
fn test_shipped_clock_css_gives_the_calendar_embed_a_280_wide_host() {
    // Issue #9: the popup measured ~240 wide even though `clock.css` sets
    // `.calendar { min-width: 280px }` on the embed node. Resolve the real
    // shipped sheets against a real Module node and lay it out with the
    // child reporting its own 240 default.
    struct CalendarMeasurer;
    impl TextMeasurer for CalendarMeasurer {
        fn measure(&mut self, _t: &str, _f: Option<&FontFamily>, _s: Option<FontSize>) -> Size {
            Size::new(0, 0)
        }
        fn measure_module(&self, _key: &crate::shared::primitives::ModuleKey) -> Option<Size> {
            Some(Size::new(240, 190))
        }
    }
    let parser = LightningCssAdapter::new();
    let sheets = [
        ("base", include_str!("../../../../../../assets/styles/base.css")),
        ("clock", include_str!("../../../../../../assets/styles/clock.css")),
    ]
    .into_iter()
    .map(|(name, css)| {
        parser
            .parse_stylesheet(StyleSheetName::new(name).unwrap(), css)
            .unwrap()
    })
    .collect();
    let resolver = CompositeStyleResolver::new(sheets);

    let embed = VNode::new_module(
        crate::features::vdom::domain::ModuleParams::new(
            crate::shared::primitives::ModuleName::new("calendar".to_string()),
            None,
            crate::shared::primitives::ModuleOptions::default(),
        ),
        Some(ClassNameList::parse("calendar").unwrap()),
        None,
        None,
        None,
        None,
    );
    let styled = embed.resolve_styles(&resolver, None, None);
    assert_eq!(styled.style().min_width(), Some(CssLength::Px(280.0)));

    let mut engine = crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter::new();
    let node = engine
        .calculate_layout(styled, &mut CalendarMeasurer, Position::new(0, 0))
        .unwrap();
    assert_eq!(node.rect().width(), 280);
}
