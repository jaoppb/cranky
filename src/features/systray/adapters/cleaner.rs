use super::icon_resolver::resolve_pixmap_data;
use crate::features::systray::domain::{
    IconName, SystrayIcon, SystrayTooltip, SystrayTooltipDescription, SystrayTooltipTitle,
};

pub type RawTooltip = (String, Vec<(i32, i32, Vec<u8>)>, String, String);

#[must_use]
pub fn clean_sni_text(input: &str) -> String {
    let s = input
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<BR>", "\n")
        .replace("<BR/>", "\n")
        .replace("<BR />", "\n");
    let mut clean = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            clean.push(c);
        }
    }
    clean
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#10;", "\n")
        .trim()
        .to_string()
}

pub fn parse_raw_tooltip(v: zbus::zvariant::OwnedValue) -> Option<SystrayTooltip> {
    if let Ok((icon_name, pixmap, title, description)) = RawTooltip::try_from(v) {
        let icon_name_opt = if icon_name.is_empty() {
            None
        } else {
            Some(icon_name)
        };
        let icon_img_opt = resolve_pixmap_data(&pixmap, 3.0);
        let tooltip_icon = SystrayIcon::new(
            icon_name_opt.map(IconName::new),
            icon_img_opt,
        );
        Some(SystrayTooltip::new(
            tooltip_icon,
            SystrayTooltipTitle::new(clean_sni_text(&title)),
            SystrayTooltipDescription::new(clean_sni_text(&description)),
        ))
    } else {
        None
    }
}
