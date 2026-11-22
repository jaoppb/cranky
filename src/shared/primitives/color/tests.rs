#[cfg(test)]
use crate::shared::primitives::color::parser::tokenize;
use crate::shared::primitives::color::{Color, ColorError, DrawingColor};

#[test]
fn test_color_new() {
    let color = Color::rgb(1, 2, 3).with_alpha(4);
    assert_eq!(color.red(), 1);
    assert_eq!(color.green(), 2);
    assert_eq!(color.blue(), 3);
    assert_eq!(color.alpha(), 4);
}

#[test]
fn test_parse_hex() {
    let color = DrawingColor::parse("#ff0000").unwrap();
    assert_eq!(color, DrawingColor::Solid(Color::rgb(255, 0, 0)));

    let color = DrawingColor::parse("#00ff00ff").unwrap();
    assert_eq!(color, DrawingColor::Solid(Color::rgb(0, 255, 0)));
}

#[test]
fn test_parse_rgb_rgba() {
    let color = DrawingColor::parse("rgb(ffffff)").unwrap();
    assert_eq!(color, DrawingColor::Solid(Color::rgb(255, 255, 255)));

    let color = DrawingColor::parse("rgba(00000080)").unwrap();
    assert_eq!(
        color,
        DrawingColor::Solid(Color::rgb(0, 0, 0).with_alpha(128))
    );
}

#[test]
fn test_parse_css_rgb_rgba() {
    let color = DrawingColor::parse("rgb(255, 128, 0)").unwrap();
    assert_eq!(color, DrawingColor::Solid(Color::rgb(255, 128, 0)));

    let color = DrawingColor::parse("rgba(255, 128, 0, 0.5)").unwrap();
    assert_eq!(
        color,
        DrawingColor::Solid(Color::rgb(255, 128, 0).with_alpha(128))
    );
}

#[test]
fn test_parse_gradient() {
    let color = DrawingColor::parse("#ff0000 #00ff00 90deg").unwrap();
    if let DrawingColor::Gradient(colors, angle) = color {
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[0], Color::rgb(255, 0, 0));
        assert_eq!(colors[1], Color::rgb(0, 255, 0));
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

#[test]
fn test_parse_hex_rejects_digits_that_are_not_hex() {
    for input in [
        "#gg0000",
        "#00gg00",
        "#0000gg",
        "#aé000",
        "#gg000000",
        "#00gg0000",
        "#0000gg00",
        "#000000gg",
        "#aé00000",
    ] {
        assert_eq!(
            DrawingColor::parse(input),
            Err(ColorError::InvalidFormat(input.to_string())),
            "{input}"
        );
    }
}

#[test]
fn test_parse_hex_rejects_lengths_other_than_six_or_eight() {
    for input in ["#12345", "#1234567", "#123456789"] {
        assert_eq!(
            DrawingColor::parse(input),
            Err(ColorError::InvalidFormat(input.to_string())),
            "{input}"
        );
    }
}

#[test]
fn test_parse_hex_reads_the_alpha_of_eight_digits() {
    let color = DrawingColor::parse("#11223344").unwrap();
    assert_eq!(
        color,
        DrawingColor::Solid(Color::rgb(0x11, 0x22, 0x33).with_alpha(0x44))
    );
}

#[test]
fn test_parse_gradient_without_an_angle_runs_at_zero_degrees() {
    let color = DrawingColor::parse("#ff0000 #00ff00").unwrap();
    assert_eq!(
        color,
        DrawingColor::Gradient(vec![Color::rgb(255, 0, 0), Color::rgb(0, 255, 0)], 0.0)
    );
}
