//! Parses grid track sizes and track lists for the grid template properties.
use crate::features::styling::domain::GridTrack;

#[must_use]
pub(crate) fn parse_single_track(text: &str) -> Option<GridTrack> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let track = keyword_track(text)
        .or_else(|| sized_track(text))
        .or_else(|| minmax_track(text));
    if track.is_none() {
        warn_if_font_relative(text);
    }
    track
}

/// `auto`, `min-content` or `max-content`, in any case.
const fn keyword_track(text: &str) -> Option<GridTrack> {
    if text.eq_ignore_ascii_case("auto") {
        Some(GridTrack::Auto)
    } else if text.eq_ignore_ascii_case("min-content") {
        Some(GridTrack::MinContent)
    } else if text.eq_ignore_ascii_case("max-content") {
        Some(GridTrack::MaxContent)
    } else {
        None
    }
}

/// A fraction, a percentage, or pixels with or without their `px`, checked in that order.
fn sized_track(text: &str) -> Option<GridTrack> {
    number_before(text, "fr")
        .map(GridTrack::Fraction)
        .or_else(|| number_before(text, "%").map(GridTrack::Percent))
        .or_else(|| number_before(text, "px").map(GridTrack::Pixels))
        .or_else(|| text.parse::<f32>().ok().map(GridTrack::Pixels))
}

/// The number written before `unit`, spaces between them allowed.
fn number_before(text: &str, unit: &str) -> Option<f32> {
    text.strip_suffix(unit)
        .and_then(|rest| rest.trim().parse::<f32>().ok())
}

/// `minmax(min, max)`, `None` unless both bounds are tracks themselves.
fn minmax_track(text: &str) -> Option<GridTrack> {
    let (min_str, max_str) = text
        .strip_prefix("minmax(")
        .and_then(|remainder| remainder.strip_suffix(')'))
        .and_then(|inner| inner.split_once(','))?;
    let min_t = parse_single_track(min_str.trim())?;
    let max_t = parse_single_track(max_str.trim())?;
    Some(GridTrack::MinMax(Box::new(min_t), Box::new(max_t)))
}

/// Warns when an unparsed track is an `em`/`rem` length.
///
/// `em`/`rem` grid tracks aren't resolved — that needs a per-element
/// font-size, but grid tracks are parsed into a plain `Vec<GridTrack>` with
/// no cascade-time resolution pass, unlike every other length-bearing
/// property in this renderer. Warn instead of silently dropping the
/// track, so the gap is visible rather than a track that just vanishes.
fn warn_if_font_relative(text: &str) {
    if let Some(rest) = text.strip_suffix("rem").or_else(|| text.strip_suffix("em"))
        && rest.trim().parse::<f32>().is_ok()
    {
        log_font_relative_track(text);
    }
}

/// Warns that an `em`/`rem` grid track is ignored.
fn log_font_relative_track(text: &str) {
    tracing::warn!(
        value = %text,
        "em/rem is not supported in grid track sizes; ignoring this track"
    );
}

#[must_use]
pub(crate) fn parse_track_list(text: &str) -> Vec<GridTrack> {
    split_tracks(text)
        .iter()
        .filter_map(|token| repeat_track(token).or_else(|| parse_single_track(token)))
        .collect()
}

/// The whitespace-separated tokens of a track list, keeping whitespace inside parentheses.
fn split_tracks(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut paren_depth = 0usize;
    for character in text.chars() {
        if character.is_whitespace() && paren_depth == 0 {
            push_token(&mut tokens, &mut current_token);
        } else {
            paren_depth = depth_after(character, paren_depth);
            current_token.push(character);
        }
    }
    push_token(&mut tokens, &mut current_token);
    tokens
}

/// The parenthesis depth after `character`, never below zero.
const fn depth_after(character: char, depth: usize) -> usize {
    match character {
        '(' => depth.saturating_add(1),
        ')' => depth.saturating_sub(1),
        _ => depth,
    }
}

/// Moves `current_token`, trimmed, into `tokens` unless it is blank.
fn push_token(tokens: &mut Vec<String>, current_token: &mut String) {
    if !current_token.trim().is_empty() {
        tokens.push(current_token.trim().to_string());
        current_token.clear();
    }
}

/// `repeat(count, tracks)` with a numeric count and at least one track it can parse.
fn repeat_track(token: &str) -> Option<GridTrack> {
    let (count_str, track_str) = token
        .strip_prefix("repeat(")
        .and_then(|remainder| remainder.strip_suffix(')'))
        .and_then(|inner| inner.split_once(','))?;
    let count = count_str.trim().parse::<u16>().ok()?;
    let sub_tracks = parse_track_list(track_str.trim());
    (!sub_tracks.is_empty()).then_some(GridTrack::Repeat(count, sub_tracks))
}
