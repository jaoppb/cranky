use crate::features::systray::adapters::cleaner::{clean_sni_text, parse_raw_tooltip, RawTooltip};
use crate::features::systray::adapters::sni_event::SniEvent;
use zbus::zvariant::Value;

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
    let val = Value::from(raw_tuple).try_into().unwrap();
    let tooltip = parse_raw_tooltip(val).expect("Should parse tooltip");
    assert_eq!(tooltip.title().as_str(), "Title");
    assert_eq!(tooltip.description().as_str(), "Description\nLine 2");
}
