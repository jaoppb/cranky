use super::*;

#[test]
fn test_window_address() {
    let addr = WindowAddress::new("0x1234");
    assert_eq!(addr.as_str(), "0x1234");
}

#[test]
fn test_window_title() {
    let title = WindowTitle::new("Firefox");
    assert_eq!(title.as_str(), "Firefox");
}

#[test]
fn test_pointer_button_conversion() {
    for (raw, button) in [
        (0x110, PointerButton::Left),
        (0x111, PointerButton::Right),
        (0x112, PointerButton::Middle),
        (0x113, PointerButton::Side),
        (0x114, PointerButton::Extra),
        (0x115, PointerButton::Forward),
        (0x116, PointerButton::Back),
        (999, PointerButton::Other(999)),
    ] {
        assert_eq!(PointerButton::from_raw(raw), button, "from_raw({raw:#x})");
        assert_eq!(button.to_raw(), raw, "{button:?}.to_raw()");
    }
}

#[test]
fn test_pointer_button_from_name() {
    for (name, button) in [
        ("left", Some(PointerButton::Left)),
        ("RIGHT", Some(PointerButton::Right)),
        ("middle", Some(PointerButton::Middle)),
        ("side", Some(PointerButton::Side)),
        ("extra", Some(PointerButton::Extra)),
        ("forward", Some(PointerButton::Forward)),
        ("back", Some(PointerButton::Back)),
        ("275", Some(PointerButton::Side)),
        ("0x113", Some(PointerButton::Side)),
        ("999", Some(PointerButton::Other(999))),
        ("unknown_btn", None),
    ] {
        assert_eq!(
            PointerButton::from_name(name),
            button,
            "from_name({name:?})"
        );
    }
}

#[test]
fn test_scroll_delta() {
    let delta = ScrollDelta::new(15.5);
    assert!((delta.value() - 15.5).abs() < f64::EPSILON);
}

#[test]
fn test_pointer_serial() {
    let serial = PointerSerial::new(1042);
    assert_eq!(serial.value(), 1042);
    assert_eq!(serial.to_string(), "1042");
}
