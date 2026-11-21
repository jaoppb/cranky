use crate::features::styling::domain::{GridLinePlacement, GridPlacement};

#[must_use]
pub fn parse_single_placement(text: &str) -> GridPlacement {
    let text = text.trim();
    if text.is_empty() || text.eq_ignore_ascii_case("auto") {
        return GridPlacement::Auto;
    }
    if let Some(rest) = text.strip_prefix("span ")
        && let Ok(count) = rest.trim().parse::<u16>()
    {
        return GridPlacement::Span(count);
    }
    if let Ok(line) = text.parse::<i16>() {
        return GridPlacement::Line(line);
    }
    GridPlacement::Auto
}

#[must_use]
pub fn parse_line_placement(text: &str) -> GridLinePlacement {
    let text = text.trim();
    if let Some((start_s, end_s)) = text.split_once('/') {
        let start = parse_single_placement(start_s);
        let end = parse_single_placement(end_s);
        GridLinePlacement::new(start, end)
    } else {
        let placement = parse_single_placement(text);
        GridLinePlacement::new(placement, GridPlacement::Auto)
    }
}
