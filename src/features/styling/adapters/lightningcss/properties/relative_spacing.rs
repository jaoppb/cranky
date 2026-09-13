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
pub(super) fn apply_spacing(lengths: &mut DeclaredLengths, prop: &Property) {
    match prop {
        Property::Padding(p) => {
            relative(&p.top).inspect(|&v| lengths.set_padding_top(v));
            relative(&p.right).inspect(|&v| lengths.set_padding_right(v));
            relative(&p.bottom).inspect(|&v| lengths.set_padding_bottom(v));
            relative(&p.left).inspect(|&v| lengths.set_padding_left(v));
        }
        Property::PaddingTop(v) => {
            relative(v).inspect(|&v| lengths.set_padding_top(v));
        }
        Property::PaddingRight(v) => {
            relative(v).inspect(|&v| lengths.set_padding_right(v));
        }
        Property::PaddingBottom(v) => {
            relative(v).inspect(|&v| lengths.set_padding_bottom(v));
        }
        Property::PaddingLeft(v) => {
            relative(v).inspect(|&v| lengths.set_padding_left(v));
        }
        Property::Margin(m) => {
            relative(&m.top).inspect(|&v| lengths.set_margin_top(v));
            relative(&m.right).inspect(|&v| lengths.set_margin_right(v));
            relative(&m.bottom).inspect(|&v| lengths.set_margin_bottom(v));
            relative(&m.left).inspect(|&v| lengths.set_margin_left(v));
        }
        Property::MarginTop(v) => {
            relative(v).inspect(|&v| lengths.set_margin_top(v));
        }
        Property::MarginRight(v) => {
            relative(v).inspect(|&v| lengths.set_margin_right(v));
        }
        Property::MarginBottom(v) => {
            relative(v).inspect(|&v| lengths.set_margin_bottom(v));
        }
        Property::MarginLeft(v) => {
            relative(v).inspect(|&v| lengths.set_margin_left(v));
        }
        _ => {}
    }
}
