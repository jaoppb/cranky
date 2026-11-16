use super::declared;
use crate::features::layout_engine::domain::{Gap, JustifyContent};
use crate::features::styling::domain::{DisplayMode, GridAutoFlow, GridTrack};

#[test]
fn display_maps_the_three_supported_modes() {
    assert_eq!(
        declared("display: grid;").display(),
        Some(DisplayMode::Grid)
    );
    assert_eq!(
        declared("display: flex;").display(),
        Some(DisplayMode::Flex)
    );
    assert_eq!(
        declared("display: none;").display(),
        Some(DisplayMode::None)
    );
    assert_eq!(declared("display: block;").display(), None);
}

#[test]
fn gap_longhand_sets_only_its_own_axis() {
    let column = declared("column-gap: 12px;");
    assert_eq!(column.column_gap(), Some(&Gap::new(12.0)));
    assert_eq!(column.row_gap(), None);

    let row = declared("row-gap: 8px;");
    assert_eq!(row.row_gap(), Some(&Gap::new(8.0)));
    assert_eq!(row.column_gap(), None);
}

#[test]
fn track_list_keeps_every_track_kind_in_order() {
    let style = declared(
        "grid-template-columns: 100px 25% 2fr auto min-content max-content minmax(10px, 1fr);",
    );
    assert_eq!(
        style.grid_template_columns(),
        Some(
            [
                GridTrack::Px(100.0),
                GridTrack::Percent(25.0),
                GridTrack::Fr(2.0),
                GridTrack::Auto,
                GridTrack::MinContent,
                GridTrack::MaxContent,
                GridTrack::MinMax(Box::new(GridTrack::Px(10.0)), Box::new(GridTrack::Fr(1.0))),
            ]
            .as_slice()
        )
    );
}

#[test]
fn repeat_keeps_its_count_and_inner_tracks() {
    let style = declared("grid-template-rows: repeat(3, 10px 1fr) auto;");
    assert_eq!(
        style.grid_template_rows(),
        Some(
            [
                GridTrack::Repeat(3, vec![GridTrack::Px(10.0), GridTrack::Fr(1.0)]),
                GridTrack::Auto,
            ]
            .as_slice()
        )
    );
}

#[test]
fn auto_tracks_go_to_their_own_axis() {
    let style = declared("grid-auto-columns: 30px; grid-auto-rows: 1fr;");
    assert_eq!(
        style.grid_auto_columns(),
        Some([GridTrack::Px(30.0)].as_slice())
    );
    assert_eq!(
        style.grid_auto_rows(),
        Some([GridTrack::Fr(1.0)].as_slice())
    );
}

#[test]
fn grid_auto_flow_maps_direction_and_density() {
    let cases = [
        ("row", GridAutoFlow::Row),
        ("column", GridAutoFlow::Column),
        ("row dense", GridAutoFlow::RowDense),
        ("column dense", GridAutoFlow::ColumnDense),
    ];
    for (keyword, expected) in cases {
        let style = declared(&format!("grid-auto-flow: {keyword};"));
        assert_eq!(style.grid_auto_flow(), Some(expected), "{keyword}");
    }
}

#[test]
fn align_content_maps_every_keyword() {
    let cases = [
        ("space-between", JustifyContent::SpaceBetween),
        ("space-around", JustifyContent::SpaceAround),
        ("space-evenly", JustifyContent::SpaceEvenly),
        ("center", JustifyContent::Center),
        ("end", JustifyContent::End),
        ("start", JustifyContent::Start),
    ];
    for (keyword, expected) in cases {
        let style = declared(&format!("align-content: {keyword};"));
        assert_eq!(style.align_content(), Some(expected), "{keyword}");
    }
}

#[test]
fn font_relative_tracks_are_dropped_and_the_rest_keep_their_order() {
    // Grid tracks have no cascade-time em/rem resolution, so such a track
    // is left out rather than mis-sized as pixels.
    let style = declared("grid-template-columns: 2em 10px 3rem 1fr;");
    assert_eq!(
        style.grid_template_columns(),
        Some([GridTrack::Px(10.0), GridTrack::Fr(1.0)].as_slice())
    );
}

#[test]
fn repeat_of_only_unsupported_tracks_contributes_nothing() {
    let style = declared("grid-template-columns: repeat(2, 1em) 10px;");
    assert_eq!(
        style.grid_template_columns(),
        Some([GridTrack::Px(10.0)].as_slice())
    );
}
