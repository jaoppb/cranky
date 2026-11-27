use crate::features::styling::adapters::lightningcss::properties::grid::tracks::{
    parse_single_track, parse_track_list,
};
use crate::features::styling::domain::GridTrack;

#[test]
fn single_track_keywords_ignore_case_and_blank_is_none() {
    assert_eq!(parse_single_track("  "), None);
    assert_eq!(parse_single_track("AUTO"), Some(GridTrack::Auto));
    assert_eq!(
        parse_single_track("Min-Content"),
        Some(GridTrack::MinContent)
    );
    assert_eq!(
        parse_single_track("MAX-content"),
        Some(GridTrack::MaxContent)
    );
}

#[test]
fn single_track_units_and_unitless_pixels() {
    assert_eq!(
        parse_single_track(" 1.5 fr "),
        Some(GridTrack::Fraction(1.5))
    );
    assert_eq!(parse_single_track("40 %"), Some(GridTrack::Percent(40.0)));
    assert_eq!(parse_single_track("12 px"), Some(GridTrack::Pixels(12.0)));
    assert_eq!(parse_single_track("7"), Some(GridTrack::Pixels(7.0)));
    assert_eq!(parse_single_track("xfr"), None);
    assert_eq!(parse_single_track("x%"), None);
    assert_eq!(parse_single_track("xpx"), None);
}

#[test]
fn single_track_minmax_needs_both_bounds() {
    assert_eq!(
        parse_single_track("minmax( auto , 3fr )"),
        Some(GridTrack::MinMax(
            Box::new(GridTrack::Auto),
            Box::new(GridTrack::Fraction(3.0))
        ))
    );
    assert_eq!(parse_single_track("minmax(1em, 1fr)"), None);
    assert_eq!(parse_single_track("minmax(10px, 1em)"), None);
    assert_eq!(parse_single_track("minmax(10px 1fr)"), None);
    assert_eq!(parse_single_track("minmax(10px, 1fr"), None);
}

#[test]
fn single_track_font_relative_and_unknown_are_none() {
    assert_eq!(parse_single_track("2em"), None);
    assert_eq!(parse_single_track("3rem"), None);
    assert_eq!(parse_single_track("fit-content"), None);
}

#[test]
fn track_list_splits_on_whitespace_outside_parentheses() {
    assert_eq!(
        parse_track_list("  10px\t minmax(1px,  2fr)\n auto  "),
        vec![
            GridTrack::Pixels(10.0),
            GridTrack::MinMax(
                Box::new(GridTrack::Pixels(1.0)),
                Box::new(GridTrack::Fraction(2.0))
            ),
            GridTrack::Auto,
        ]
    );
    assert_eq!(parse_track_list(""), Vec::<GridTrack>::new());
}

#[test]
fn track_list_repeat_nests_and_keeps_its_count() {
    assert_eq!(
        parse_track_list("repeat(2, 1fr repeat(3, 5px))"),
        vec![GridTrack::Repeat(
            2,
            vec![
                GridTrack::Fraction(1.0),
                GridTrack::Repeat(3, vec![GridTrack::Pixels(5.0)])
            ]
        )]
    );
}

#[test]
fn track_list_drops_a_repeat_it_cannot_count_or_fill() {
    assert_eq!(
        parse_track_list("repeat(auto-fill, 10px) repeat(2, 1em) repeat(3 10px) 4px"),
        vec![GridTrack::Pixels(4.0)]
    );
}

#[test]
fn track_list_tolerates_an_unbalanced_closing_parenthesis() {
    assert_eq!(parse_track_list(") 5px"), vec![GridTrack::Pixels(5.0)]);
}
