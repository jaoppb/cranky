use crate::features::systray::adapters::cleaner::{RawTooltip, clean_sni_text, parse_raw_tooltip};
use crate::features::systray::adapters::sni_event::SniEvent;
use crate::features::systray::domain::{IconImage, IconName};
use zbus::Variant;

#[test]
fn test_sni_event_variants() {
    let _events = [
        SniEvent::Title,
        SniEvent::Status("Active".to_string()),
        SniEvent::Icon,
        SniEvent::ThemePath,
        SniEvent::AttentionIcon,
        SniEvent::OverlayIcon,
    ];
}

#[test]
fn test_clean_sni_text_strips_html_and_converts_br() {
    let raw = "<b>Ducking ON</b><br/>Audio: &lt;enabled&gt; &amp; active";
    let cleaned = clean_sni_text(raw);
    assert_eq!(cleaned, "Ducking ON\nAudio: <enabled> & active");
}

#[test]
fn test_parse_raw_tooltip() {
    let raw_tuple: RawTooltip = (
        "test-icon".to_string(),
        vec![],
        "<b>Title</b>".to_string(),
        "Description<br/>Line 2".to_string(),
    );
    let val = Variant::from(raw_tuple).try_into().unwrap();
    let tooltip = parse_raw_tooltip(val).expect("Should parse tooltip");
    assert_eq!(tooltip.title().as_str(), "Title");
    assert_eq!(tooltip.description().as_str(), "Description\nLine 2");
}

#[test]
fn test_clean_sni_text_converts_every_br_spelling() {
    let raw = "a<br>b<br/>c<br />d<BR>e<BR/>f<BR />g";
    assert_eq!(clean_sni_text(raw), "a\nb\nc\nd\ne\nf\ng");
}

#[test]
fn test_clean_sni_text_decodes_entities_after_stripping_tags() {
    // An escaped tag survives as text: entities are decoded only once the tags are gone.
    assert_eq!(clean_sni_text("&lt;b&gt;x&lt;/b&gt;"), "<b>x</b>");
    // `&amp;` is decoded first, so `&amp;lt;` ends up as `<`.
    assert_eq!(clean_sni_text("&amp;lt;"), "<");
    assert_eq!(clean_sni_text("a&quot;b&apos;c&#10;d"), "a\"b'c\nd");
}

#[test]
fn test_clean_sni_text_drops_brackets_and_trims() {
    assert_eq!(clean_sni_text("  a > b  "), "a  b");
    assert_eq!(clean_sni_text("a <b c"), "a");
    assert_eq!(clean_sni_text("\n<i></i>\n"), "");
}

#[test]
fn test_parse_raw_tooltip_keeps_icon_name_and_pixmap() {
    let raw_tuple: RawTooltip = (
        "tip-icon".to_string(),
        vec![(1, 1, vec![255, 10, 20, 30])],
        "T".to_string(),
        "D".to_string(),
    );
    let tooltip = parse_raw_tooltip(Variant::from(raw_tuple).try_into().unwrap()).unwrap();
    let icon = tooltip.icon().expect("the tooltip has an icon");
    assert_eq!(icon.name().map(IconName::as_str), Some("tip-icon"));
    assert_eq!(
        icon.image().map(IconImage::data),
        Some(&[10, 20, 30, 255][..])
    );
}

#[test]
fn test_parse_raw_tooltip_without_icon_name_or_pixmap_has_no_icon() {
    let raw_tuple: RawTooltip = (String::new(), vec![], "T".to_string(), "D".to_string());
    let tooltip = parse_raw_tooltip(Variant::from(raw_tuple).try_into().unwrap()).unwrap();
    assert_eq!(tooltip.icon(), None);
    assert_eq!(
        (tooltip.title().as_str(), tooltip.description().as_str()),
        ("T", "D")
    );
}

#[test]
fn test_parse_raw_tooltip_rejects_another_signature() {
    assert_eq!(
        parse_raw_tooltip(Variant::from("text").try_into().unwrap()),
        None
    );
}
