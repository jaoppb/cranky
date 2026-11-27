//! Fixtures the lightningcss adapter tests share: a sheet parsed from CSS, the style it
//! gives one element, and the text a laid-out tree paints.
use crate::features::layout_engine::domain::RenderNode;
use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::{
    ClassName, ComputedStyle, ElementIdentity, ElementQuery, InheritedStyle, StyleSheetName,
};
use crate::features::styling::ports::{CssParserPort, ParsedStyleSheetPort};
use crate::shared::primitives::TextAlignment;
use crate::shared::rendering::ports::canvas::{CanvasCall, RecordingCanvas};

/// The sheet `css` parses into under the stylesheet name `name`, failing the test on a parse
/// error.
pub(super) fn parse(name: &str, css: &str) -> Box<dyn ParsedStyleSheetPort> {
    LightningCssAdapter::new()
        .parse_stylesheet(StyleSheetName::new(name).unwrap(), css)
        .unwrap()
}

/// The style `sheet` gives `query` under the default inherited style, dropping what it would
/// pass on to children.
pub(super) fn resolve(sheet: &dyn ParsedStyleSheetPort, query: &ElementQuery<'_>) -> ComputedStyle {
    sheet.resolve_style(query, &InheritedStyle::default()).0
}

/// The style `sheet` gives a `tag` element carrying the one class `class`, with no
/// pseudo-class and no parent.
pub(super) fn class_style(
    sheet: &dyn ParsedStyleSheetPort,
    tag: &str,
    class: &str,
) -> ComputedStyle {
    let class = ClassName::new(class).unwrap();
    let identity = ElementIdentity::new(tag, None, std::slice::from_ref(&class));
    resolve(sheet, &ElementQuery::new(identity, &[], None))
}

/// Every `draw_text` call `laid_out` paints, as `(text, alignment)` in paint order.
pub(super) fn painted_texts(laid_out: &RenderNode) -> Vec<(String, TextAlignment)> {
    let mut canvas = RecordingCanvas::default();
    laid_out.render_to_canvas(&mut canvas);
    canvas
        .calls
        .into_iter()
        .filter_map(|call| match call {
            CanvasCall::Text {
                text, alignment, ..
            } => Some((text, alignment)),
            CanvasCall::Rect { .. } | CanvasCall::Border { .. } | CanvasCall::Image { .. } => None,
        })
        .collect()
}
