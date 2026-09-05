use crate::features::styling::domain::GridTrack;

#[must_use]
pub fn parse_single_track(s: &str) -> Option<GridTrack> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if s.eq_ignore_ascii_case("auto") {
        return Some(GridTrack::Auto);
    }
    if s.eq_ignore_ascii_case("min-content") {
        return Some(GridTrack::MinContent);
    }
    if s.eq_ignore_ascii_case("max-content") {
        return Some(GridTrack::MaxContent);
    }
    if let Some(rest) = s.strip_suffix("fr")
        && let Ok(v) = rest.trim().parse::<f32>()
    {
        return Some(GridTrack::Fr(v));
    }
    if let Some(rest) = s.strip_suffix('%')
        && let Ok(v) = rest.trim().parse::<f32>()
    {
        return Some(GridTrack::Percent(v));
    }
    if let Some(rest) = s.strip_suffix("px")
        && let Ok(v) = rest.trim().parse::<f32>()
    {
        return Some(GridTrack::Px(v));
    }
    if let Ok(v) = s.parse::<f32>() {
        return Some(GridTrack::Px(v));
    }
    if let Some(inner) = s.strip_prefix("minmax(").and_then(|r| r.strip_suffix(')'))
        && let Some((min_str, max_str)) = inner.split_once(',')
    {
        let min_t = parse_single_track(min_str.trim())?;
        let max_t = parse_single_track(max_str.trim())?;
        return Some(GridTrack::MinMax(Box::new(min_t), Box::new(max_t)));
    }
    None
}

#[must_use]
pub fn parse_track_list(s: &str) -> Vec<GridTrack> {
    let mut tracks = Vec::new();
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut paren_depth = 0usize;

    for c in s.chars() {
        if c == '(' {
            paren_depth = paren_depth.saturating_add(1);
            current_token.push(c);
        } else if c == ')' {
            paren_depth = paren_depth.saturating_sub(1);
            current_token.push(c);
        } else if c.is_whitespace() && paren_depth == 0 {
            if !current_token.trim().is_empty() {
                tokens.push(current_token.trim().to_string());
                current_token.clear();
            }
        } else {
            current_token.push(c);
        }
    }
    if !current_token.trim().is_empty() {
        tokens.push(current_token.trim().to_string());
    }

    for token in tokens {
        if let Some(inner) = token
            .strip_prefix("repeat(")
            .and_then(|r| r.strip_suffix(')'))
            && let Some((count_str, track_str)) = inner.split_once(',')
            && let Ok(count) = count_str.trim().parse::<u16>()
        {
            let sub_tracks = parse_track_list(track_str.trim());
            if !sub_tracks.is_empty() {
                tracks.push(GridTrack::Repeat(count, sub_tracks));
                continue;
            }
        }
        if let Some(t) = parse_single_track(&token) {
            tracks.push(t);
        }
    }
    tracks
}
