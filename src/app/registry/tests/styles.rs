//! The style resolver a module's actor gets: `base` first, in the base layer, then the
//! module's own sheets in order, skipping any sheet that cannot be read or parsed.
use super::super::ModuleRegistry;
use crate::features::styling::domain::{
    ElementIdentity, ElementQuery, InheritedStyle, StyleSheetName,
};
use crate::shared::primitives::color::{Color, DrawingColor};

const BASE_RED: &str = "probe { background-color: #ff0000; }";
const PROBE_BLUE: &str = "probe { background-color: #0000ff; }";
/// A sheet lightningcss rejects: an `@import` after a rule.
const UNPARSEABLE: &str = "probe { color: red; } @import 'late.css';";

const RED: Color = Color::rgb(255, 0, 0);
const BLUE: Color = Color::rgb(0, 0, 255);

/// A registry over `home` whose user styles directory holds `sheets` as `(name, css)`.
fn registry_with_user_styles(
    home: &crate::test_utils::TestHome,
    sheets: &[(&str, &str)],
) -> ModuleRegistry {
    let dir = home.home().as_path().join(".config/cranky/styles");
    std::fs::create_dir_all(&dir).unwrap();
    for (name, css) in sheets {
        std::fs::write(dir.join(format!("{name}.css")), css).unwrap();
    }
    ModuleRegistry::new(home.env())
}

/// The solid background the resolver for `styles` gives a bare `probe` element, if any.
fn probe_background(registry: &ModuleRegistry, styles: &[&str]) -> Option<Color> {
    let names: Vec<StyleSheetName> = styles
        .iter()
        .map(|name| StyleSheetName::new(*name).unwrap())
        .collect();
    let resolver = registry.create_style_resolver_for_module(&names);
    let query = ElementQuery::new(ElementIdentity::new("probe", None, &[]), &[], None);
    match resolver
        .resolve_style(&query, &InheritedStyle::default())
        .0
        .background()
    {
        Some(DrawingColor::Solid(color)) => Some(*color),
        _ => None,
    }
}

#[test]
fn the_base_sheet_applies_to_a_module_with_no_sheet_of_its_own() {
    let home = crate::test_utils::TestHome::new("registry-styles");
    let registry = registry_with_user_styles(&home, &[("base", BASE_RED)]);
    assert_eq!(probe_background(&registry, &[]), Some(RED));
}

#[test]
fn a_module_sheet_overrides_the_base_sheet() {
    let home = crate::test_utils::TestHome::new("registry-styles");
    let registry = registry_with_user_styles(&home, &[("base", BASE_RED), ("probe", PROBE_BLUE)]);
    assert_eq!(probe_background(&registry, &["probe"]), Some(BLUE));
}

/// Listed after the module's sheet, `base` still yields to it: `base` stays in the base
/// layer, whatever its place in the list.
#[test]
fn base_listed_after_a_module_sheet_still_yields_to_it() {
    let home = crate::test_utils::TestHome::new("registry-styles");
    let registry = registry_with_user_styles(&home, &[("base", BASE_RED), ("probe", PROBE_BLUE)]);
    assert_eq!(probe_background(&registry, &["probe", "base"]), Some(BLUE));
}

#[test]
fn a_missing_or_unparseable_module_sheet_is_skipped_and_the_next_one_still_loads() {
    let home = crate::test_utils::TestHome::new("registry-styles");
    let sheets = [("broken", UNPARSEABLE), ("probe", PROBE_BLUE)];
    let registry = registry_with_user_styles(&home, &sheets);
    let styles = ["no-such-sheet", "broken", "probe"];
    assert_eq!(probe_background(&registry, &styles), Some(BLUE));
}

#[test]
fn an_unparseable_base_sheet_is_skipped_and_the_module_sheets_still_load() {
    let home = crate::test_utils::TestHome::new("registry-styles");
    let registry =
        registry_with_user_styles(&home, &[("base", UNPARSEABLE), ("probe", PROBE_BLUE)]);
    assert_eq!(probe_background(&registry, &[]), None);
    assert_eq!(probe_background(&registry, &["probe"]), Some(BLUE));
}
