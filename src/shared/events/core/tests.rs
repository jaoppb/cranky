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
    assert_eq!(PointerButton::from_raw(0x110), PointerButton::Left);
    assert_eq!(PointerButton::from_raw(0x111), PointerButton::Right);
    assert_eq!(PointerButton::from_raw(0x112), PointerButton::Middle);
    assert_eq!(PointerButton::from_raw(0x113), PointerButton::Side);
    assert_eq!(PointerButton::from_raw(0x114), PointerButton::Extra);
    assert_eq!(PointerButton::from_raw(0x115), PointerButton::Forward);
    assert_eq!(PointerButton::from_raw(0x116), PointerButton::Back);
    assert_eq!(PointerButton::from_raw(999), PointerButton::Other(999));

    assert_eq!(PointerButton::Left.to_raw(), 0x110);
    assert_eq!(PointerButton::Right.to_raw(), 0x111);
    assert_eq!(PointerButton::Middle.to_raw(), 0x112);
    assert_eq!(PointerButton::Side.to_raw(), 0x113);
    assert_eq!(PointerButton::Extra.to_raw(), 0x114);
    assert_eq!(PointerButton::Forward.to_raw(), 0x115);
    assert_eq!(PointerButton::Back.to_raw(), 0x116);
    assert_eq!(PointerButton::Other(999).to_raw(), 999);

    assert_eq!(PointerButton::from_name("left"), Some(PointerButton::Left));
    assert_eq!(PointerButton::from_name("RIGHT"), Some(PointerButton::Right));
    assert_eq!(
        PointerButton::from_name("middle"),
        Some(PointerButton::Middle)
    );
    assert_eq!(PointerButton::from_name("side"), Some(PointerButton::Side));
    assert_eq!(
        PointerButton::from_name("extra"),
        Some(PointerButton::Extra)
    );
    assert_eq!(
        PointerButton::from_name("forward"),
        Some(PointerButton::Forward)
    );
    assert_eq!(PointerButton::from_name("back"), Some(PointerButton::Back));
    assert_eq!(PointerButton::from_name("275"), Some(PointerButton::Side));
    assert_eq!(PointerButton::from_name("0x113"), Some(PointerButton::Side));
    assert_eq!(
        PointerButton::from_name("999"),
        Some(PointerButton::Other(999))
    );
    assert_eq!(PointerButton::from_name("unknown_btn"), None);
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

