use crate::features::styling::domain::{GridLinePlacement, GridPlacement};

#[must_use]
pub fn parse_single_placement(s: &str) -> GridPlacement {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("auto") {
        return GridPlacement::Auto;
    }
    if let Some(rest) = s.strip_prefix("span ")
        && let Ok(count) = rest.trim().parse::<u16>()
    {
        return GridPlacement::Span(count);
    }
    if let Ok(line) = s.parse::<i16>() {
        return GridPlacement::Line(line);
    }
    GridPlacement::Auto
}

#[must_use]
pub fn parse_line_placement(s: &str) -> GridLinePlacement {
    let s = s.trim();
    if let Some((start_s, end_s)) = s.split_once('/') {
        let start = parse_single_placement(start_s);
        let end = parse_single_placement(end_s);
        GridLinePlacement::new(start, end)
    } else {
        let p = parse_single_placement(s);
        GridLinePlacement::new(p, GridPlacement::Auto)
    }
}
