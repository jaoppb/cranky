#[cfg(test)]
use crate::shared::primitives::color::parser::tokenize;
use crate::shared::primitives::color::{Color, ColorError, DrawingColor};

#[test]
fn test_color_new() {
    let c = Color::new(1, 2, 3, 4);
    assert_eq!(c.r(), 1);
    assert_eq!(c.g(), 2);
    assert_eq!(c.b(), 3);
    assert_eq!(c.a(), 4);
}

#[test]
fn test_parse_hex() {
    let c = DrawingColor::parse("#ff0000").unwrap();
    assert_eq!(c, DrawingColor::Solid(Color::new(255, 0, 0, 255)));

    let c = DrawingColor::parse("#00ff00ff").unwrap();
    assert_eq!(c, DrawingColor::Solid(Color::new(0, 255, 0, 255)));
}

#[test]
fn test_parse_rgb_rgba() {
    let c = DrawingColor::parse("rgb(ffffff)").unwrap();
    assert_eq!(c, DrawingColor::Solid(Color::new(255, 255, 255, 255)));

    let c = DrawingColor::parse("rgba(00000080)").unwrap();
    assert_eq!(c, DrawingColor::Solid(Color::new(0, 0, 0, 128)));
}

#[test]
fn test_parse_css_rgb_rgba() {
    let c = DrawingColor::parse("rgb(255, 128, 0)").unwrap();
    assert_eq!(c, DrawingColor::Solid(Color::new(255, 128, 0, 255)));

    let c = DrawingColor::parse("rgba(255, 128, 0, 0.5)").unwrap();
    assert_eq!(c, DrawingColor::Solid(Color::new(255, 128, 0, 128)));
}

#[test]
fn test_parse_gradient() {
    let c = DrawingColor::parse("#ff0000 #00ff00 90deg").unwrap();
    if let DrawingColor::Gradient(colors, angle) = c {
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[0], Color::new(255, 0, 0, 255));
        assert_eq!(colors[1], Color::new(0, 255, 0, 255));
        assert!((angle - 90.0).abs() < f32::EPSILON);
    } else {
        panic!("Expected gradient");
    }
}

#[test]
fn test_parse_errors() {
    assert_eq!(DrawingColor::parse("  "), Err(ColorError::Empty));
    assert!(DrawingColor::parse("not-a-color").is_err());
    assert!(matches!(
        DrawingColor::parse("#ff0000 #00ff00 invalid"),
        Err(ColorError::InvalidAngle(_))
    ));
}

#[test]
fn test_tokenize() {
    let tokens = tokenize("rgb(255, 255, 255) #000");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], "rgb(255, 255, 255)");
}
