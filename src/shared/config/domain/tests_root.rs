//! Unit tests for the `[root.unfocused]` overrides of the root config.
use super::*;

/// A root config 30 high, centred, with margins 10/20/30/40, carrying `unfocused`.
fn root_with_unfocused(unfocused: PartialRootConfig) -> RootConfig {
    RootConfig::default()
        .with_margin(MarginConfig::new(
            AxisMargins::new(MarginOffset::new(10), MarginOffset::new(20)),
            AxisMargins::new(MarginOffset::new(30), MarginOffset::new(40)),
        ))
        .with_unfocused(Some(unfocused))
}

#[test]
fn test_root_config_as_unfocused_applies_every_override_it_sets() {
    let unfocused = PartialRootConfig::new(CreatePartialRootConfigCommand::new(
        Some(crate::shared::primitives::geometry::BarHeight::new(24)),
        Some(VerticalAlignment::Bottom),
        Some(PartialMarginConfig::new(
            AxisMargins::new(Some(MarginOffset::new(1)), None),
            AxisMargins::new(None, Some(MarginOffset::new(4))),
        )),
    ));

    let root = root_with_unfocused(unfocused).as_unfocused();

    assert_eq!(root.height().value(), 24);
    assert_eq!(root.vertical_alignment(), VerticalAlignment::Bottom);
    let margin = root.margin();
    assert_eq!(
        [margin.top(), margin.bottom(), margin.left(), margin.right()].map(MarginOffset::value),
        [1, 20, 30, 4]
    );
}

#[test]
fn test_root_config_as_unfocused_keeps_the_base_where_nothing_is_set() {
    let unfocused = PartialRootConfig::new(CreatePartialRootConfigCommand::new(None, None, None));
    let base = root_with_unfocused(unfocused);

    let root = base.as_unfocused();

    assert_eq!(root.height().value(), 30);
    assert_eq!(root.vertical_alignment(), VerticalAlignment::Center);
    assert_eq!(root.margin(), base.margin());
}
