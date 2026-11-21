use super::relative_lengths::relative;
use crate::features::styling::domain::DeclaredLengths;
use lightningcss::properties::Property;

/// `em`/`rem` detection for `padding`/`margin`, split out of
/// `relative_lengths.rs` to stay under the 200 LOC production file limit.
/// No `calc()` support here (unlike width/height/flex-basis): these
/// properties have no percentage-mixing use case this renderer resolves
/// (they never carried `Percent` at all before `calc()` existed), so a
/// `calc()` value here still falls through to the ordinary pipeline's
/// pixel guess, same as before this stage.
pub(super) fn apply_spacing(lengths: &mut DeclaredLengths, prop: &Property<'_>) {
    match prop {
        Property::Padding(padding) => {
            relative(&padding.top).inspect(|&declared| lengths.set_padding_top(declared));
            relative(&padding.right).inspect(|&declared| lengths.set_padding_right(declared));
            relative(&padding.bottom).inspect(|&declared| lengths.set_padding_bottom(declared));
            relative(&padding.left).inspect(|&declared| lengths.set_padding_left(declared));
        }
        Property::PaddingTop(value) => {
            relative(value).inspect(|&declared| lengths.set_padding_top(declared));
        }
        Property::PaddingRight(value) => {
            relative(value).inspect(|&declared| lengths.set_padding_right(declared));
        }
        Property::PaddingBottom(value) => {
            relative(value).inspect(|&declared| lengths.set_padding_bottom(declared));
        }
        Property::PaddingLeft(value) => {
            relative(value).inspect(|&declared| lengths.set_padding_left(declared));
        }
        Property::Margin(margin) => {
            relative(&margin.top).inspect(|&declared| lengths.set_margin_top(declared));
            relative(&margin.right).inspect(|&declared| lengths.set_margin_right(declared));
            relative(&margin.bottom).inspect(|&declared| lengths.set_margin_bottom(declared));
            relative(&margin.left).inspect(|&declared| lengths.set_margin_left(declared));
        }
        Property::MarginTop(value) => {
            relative(value).inspect(|&declared| lengths.set_margin_top(declared));
        }
        Property::MarginRight(value) => {
            relative(value).inspect(|&declared| lengths.set_margin_right(declared));
        }
        Property::MarginBottom(value) => {
            relative(value).inspect(|&declared| lengths.set_margin_bottom(declared));
        }
        Property::MarginLeft(value) => {
            relative(value).inspect(|&declared| lengths.set_margin_left(declared));
        }
        _ => {}
    }
}
