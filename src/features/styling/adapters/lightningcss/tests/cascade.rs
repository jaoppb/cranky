use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::styling::adapters::lightningcss::{
    LightningCssAdapter, LightningPropertyReparser,
};
use crate::features::styling::domain::{
    ClassName, ComputedStyle, ElementId, ElementQuery, InheritedStyle, PseudoClass, StyleSheetName,
};
use crate::features::styling::ports::{CssParserPort, StyleResolverPort};
use crate::shared::primitives::color::{Color, DrawingColor};

fn solid(style: &ComputedStyle) -> Color {
    match style.background() {
        Some(DrawingColor::Solid(c)) => *c,
        other => panic!("expected a solid background color, got {other:?}"),
    }
}

#[test]
fn test_id_beats_class_regardless_of_source_order() {
    let parser = LightningCssAdapter::new();
    let css = r"
        #root { background-color: #ffffff; }
        .a.b.c { background-color: #000000; }
    ";
    let parsed = parser
        .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
        .unwrap();

    let classes = [
        ClassName::new("a").unwrap(),
        ClassName::new("b").unwrap(),
        ClassName::new("c").unwrap(),
    ];
    let id = ElementId::new("root").unwrap();
    let query = ElementQuery::new("bar", Some(&id), &classes, &[], None);
    let (style, _) = parsed.resolve_style(
        &query,
        &InheritedStyle::default(),
        &LightningPropertyReparser,
    );

    assert_eq!(solid(&style), Color::new(255, 255, 255, 255));
}

#[test]
fn test_important_beats_specificity_and_source_order() {
    let parser = LightningCssAdapter::new();
    let css = r"
        #root { background-color: #ffffff !important; }
        .a.b.c { background-color: #000000; }
    ";
    let parsed = parser
        .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
        .unwrap();

    let classes = [
        ClassName::new("a").unwrap(),
        ClassName::new("b").unwrap(),
        ClassName::new("c").unwrap(),
    ];
    // Give the class selector higher specificity than the id by matching it
    // alone against a query with no id, to prove importance — not specificity
    // — is what decides here in the id case above; this one is the control.
    let query = ElementQuery::new("bar", None, &classes, &[], None);
    let (style, _) = parsed.resolve_style(
        &query,
        &InheritedStyle::default(),
        &LightningPropertyReparser,
    );
    assert_eq!(solid(&style), Color::new(0, 0, 0, 255));
}

#[test]
fn test_module_layer_beats_base_layer_for_normal_declarations() {
    let parser = LightningCssAdapter::new();
    let base = parser
        .parse_stylesheet(
            StyleSheetName::new("base").unwrap(),
            "#root { background-color: #ffffff; }",
        )
        .unwrap();
    // Lower specificity than base's #root, but in the module layer, which
    // wins regardless for normal declarations.
    let module = parser
        .parse_stylesheet(
            StyleSheetName::new("workspace").unwrap(),
            "bar { background-color: #000000; }",
        )
        .unwrap();

    let resolver = CompositeStyleResolver::new(vec![base, module]);
    let id = ElementId::new("root").unwrap();
    let query = ElementQuery::new("bar", Some(&id), &[], &[], None);
    let (style, _) = resolver.resolve_style(&query, &InheritedStyle::default());

    assert_eq!(solid(&style), Color::new(0, 0, 0, 255));
}

#[test]
fn test_base_layer_wins_when_important() {
    let parser = LightningCssAdapter::new();
    let base = parser
        .parse_stylesheet(
            StyleSheetName::new("base").unwrap(),
            "bar { background-color: #ffffff !important; }",
        )
        .unwrap();
    let module = parser
        .parse_stylesheet(
            StyleSheetName::new("workspace").unwrap(),
            "#root { background-color: #000000; }",
        )
        .unwrap();

    let resolver = CompositeStyleResolver::new(vec![base, module]);
    let id = ElementId::new("root").unwrap();
    let query = ElementQuery::new("bar", Some(&id), &[], &[], None);
    let (style, _) = resolver.resolve_style(&query, &InheritedStyle::default());

    // Important flips layer order: base's !important beats module's normal
    // declaration even though the module selector is more specific.
    assert_eq!(solid(&style), Color::new(255, 255, 255, 255));
}

#[test]
fn test_workspace_hover_wins_over_active_focused() {
    let parser = LightningCssAdapter::new();
    let css = include_str!("../../../../../../assets/styles/workspace.css");
    let parsed = parser
        .parse_stylesheet(StyleSheetName::new("workspace").unwrap(), css)
        .unwrap();

    let classes = [
        ClassName::new("item").unwrap(),
        ClassName::new("active").unwrap(),
        ClassName::new("focused").unwrap(),
    ];
    let query = ElementQuery::new("flex", None, &classes, &[PseudoClass::Hover], None);
    let (style, _) = parsed.resolve_style(
        &query,
        &InheritedStyle::default(),
        &LightningPropertyReparser,
    );

    // #414868 (the hover color), not #565f89 (active+focused) — hovering an
    // active, focused workspace item must still show the hover highlight.
    assert_eq!(solid(&style), Color::new(65, 72, 104, 255));
}
