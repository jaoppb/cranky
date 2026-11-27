use super::icon_resolver::resolve_pixmap_data;
use crate::features::systray::domain::{
    IconName, SystrayIcon, SystrayTooltip, SystrayTooltipDescription, SystrayTooltipTitle,
};

pub(super) type RawTooltip = (String, Vec<(i32, i32, Vec<u8>)>, String, String);

/// `input` with every `<br>` spelling turned into a newline.
fn breaks_to_newlines(input: &str) -> String {
    input
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<BR>", "\n")
        .replace("<BR/>", "\n")
        .replace("<BR />", "\n")
}

/// `text` without its markup: everything from a `<` to the next `>` goes, and so does a stray `>`.
fn strip_tags(text: &str) -> String {
    let mut clean = String::with_capacity(text.len());
    let mut in_tag = false;
    for character in text.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => clean.push(character),
            _ => {}
        }
    }
    clean
}

/// `text` with its XML entities decoded, `&amp;` first, and trimmed.
fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#10;", "\n")
        .trim()
        .to_string()
}

/// The plain text of an SNI title or description, which may carry a subset of HTML: line breaks
/// kept, tags dropped, entities decoded, surrounding whitespace trimmed.
#[must_use]
pub(super) fn clean_sni_text(input: &str) -> String {
    decode_entities(&strip_tags(&breaks_to_newlines(input)))
}

pub(super) fn parse_raw_tooltip(variant: zbus::OwnedVariant) -> Option<SystrayTooltip> {
    if let Ok((icon_name, pixmap, title, description)) = RawTooltip::try_from(variant) {
        let icon_name_opt = if icon_name.is_empty() {
            None
        } else {
            Some(icon_name)
        };
        let icon_img_opt = resolve_pixmap_data(&pixmap, 3.0);
        let tooltip_icon = SystrayIcon::new(icon_name_opt.map(IconName::new), icon_img_opt);
        Some(SystrayTooltip::new(
            tooltip_icon,
            SystrayTooltipTitle::new(clean_sni_text(&title)),
            SystrayTooltipDescription::new(clean_sni_text(&description)),
        ))
    } else {
        None
    }
}
