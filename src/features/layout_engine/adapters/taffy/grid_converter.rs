use crate::features::styling::domain::{GridAutoFlow, GridLinePlacement, GridPlacement, GridTrack};
use taffy::style_helpers::{
    auto, fr, length, line, max_content, min_content, minmax, percent, repeat, span,
};

#[must_use]
pub fn grid_track_to_min_track(track: &GridTrack) -> taffy::style::MinTrackSizingFunction {
    match track {
        GridTrack::Px(px) => length(*px),
        GridTrack::Percent(pct) => percent(*pct / 100.0),
        GridTrack::MinContent => min_content(),
        GridTrack::MaxContent => max_content(),
        _ => auto(),
    }
}

#[must_use]
pub fn grid_track_to_max_track(track: &GridTrack) -> taffy::style::MaxTrackSizingFunction {
    match track {
        GridTrack::Px(px) => length(*px),
        GridTrack::Percent(pct) => percent(*pct / 100.0),
        GridTrack::Fr(fr_val) => fr(*fr_val),
        GridTrack::MinContent => min_content(),
        GridTrack::MaxContent => max_content(),
        _ => auto(),
    }
}

#[must_use]
pub fn grid_track_to_track_sizing_function(track: &GridTrack) -> taffy::style::TrackSizingFunction {
    match track {
        GridTrack::MinMax(min_t, max_t) => minmax(
            grid_track_to_min_track(min_t),
            grid_track_to_max_track(max_t),
        ),
        _ => minmax(
            grid_track_to_min_track(track),
            grid_track_to_max_track(track),
        ),
    }
}

#[must_use]
pub fn grid_track_to_template_component(
    track: &GridTrack,
) -> taffy::style::GridTemplateComponent<String> {
    match track {
        GridTrack::Repeat(count, sub_tracks) => {
            let non_rep: Vec<taffy::style::TrackSizingFunction> = sub_tracks
                .iter()
                .map(grid_track_to_track_sizing_function)
                .collect();
            repeat(*count, non_rep)
        }
        _ => {
            taffy::style::GridTemplateComponent::Single(grid_track_to_track_sizing_function(track))
        }
    }
}

#[must_use]
pub fn grid_placement_to_taffy(placement: GridPlacement) -> taffy::style::GridPlacement<String> {
    match placement {
        GridPlacement::Auto => taffy::style::GridPlacement::Auto,
        GridPlacement::Line(l) => line(l),
        GridPlacement::Span(s) => span(s),
    }
}

#[must_use]
pub fn grid_line_placement_to_taffy(
    line_placement: GridLinePlacement,
) -> taffy::geometry::Line<taffy::style::GridPlacement<String>> {
    taffy::geometry::Line {
        start: grid_placement_to_taffy(line_placement.start()),
        end: grid_placement_to_taffy(line_placement.end()),
    }
}

#[must_use]
pub const fn grid_auto_flow_to_taffy(flow: GridAutoFlow) -> taffy::style::GridAutoFlow {
    match flow {
        GridAutoFlow::Row => taffy::style::GridAutoFlow::Row,
        GridAutoFlow::Column => taffy::style::GridAutoFlow::Column,
        GridAutoFlow::RowDense => taffy::style::GridAutoFlow::RowDense,
        GridAutoFlow::ColumnDense => taffy::style::GridAutoFlow::ColumnDense,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taffy::style::{
        GridTemplateComponent, MaxTrackSizingFunction, MinTrackSizingFunction, TrackSizingFunction,
    };

    fn sizing(min: MinTrackSizingFunction, max: MaxTrackSizingFunction) -> TrackSizingFunction {
        minmax(min, max)
    }

    #[test]
    fn fixed_and_content_tracks_use_the_same_size_for_min_and_max() {
        assert_eq!(
            grid_track_to_track_sizing_function(&GridTrack::Px(40.0)),
            sizing(length(40.0_f32), length(40.0_f32))
        );
        assert_eq!(
            grid_track_to_track_sizing_function(&GridTrack::MinContent),
            sizing(min_content(), min_content())
        );
        assert_eq!(
            grid_track_to_track_sizing_function(&GridTrack::MaxContent),
            sizing(max_content(), max_content())
        );
        assert_eq!(
            grid_track_to_track_sizing_function(&GridTrack::Auto),
            sizing(auto(), auto())
        );
    }

    #[test]
    fn percent_tracks_are_converted_from_percent_to_a_fraction() {
        assert_eq!(
            grid_track_to_track_sizing_function(&GridTrack::Percent(25.0)),
            sizing(percent(0.25_f32), percent(0.25_f32))
        );
    }

    #[test]
    fn fr_tracks_flex_only_on_the_max_side() {
        // A flexible minimum is not valid in CSS grid: `1fr` means
        // `minmax(auto, 1fr)`.
        assert_eq!(
            grid_track_to_track_sizing_function(&GridTrack::Fr(2.0)),
            sizing(auto(), fr(2.0_f32))
        );
    }

    #[test]
    fn minmax_tracks_take_min_and_max_from_their_own_sides() {
        let track = GridTrack::MinMax(Box::new(GridTrack::Px(10.0)), Box::new(GridTrack::Fr(1.0)));
        assert_eq!(
            grid_track_to_track_sizing_function(&track),
            sizing(length(10.0_f32), fr(1.0_f32))
        );
    }

    #[test]
    fn repeat_tracks_become_a_repeat_component_with_the_same_count() {
        let track = GridTrack::Repeat(3, vec![GridTrack::Px(10.0), GridTrack::Fr(1.0)]);
        let expected: GridTemplateComponent<String> = repeat(
            3,
            vec![
                sizing(length(10.0_f32), length(10.0_f32)),
                sizing(auto(), fr(1.0_f32)),
            ],
        );
        assert_eq!(grid_track_to_template_component(&track), expected);
    }

    #[test]
    fn non_repeat_tracks_become_a_single_component() {
        assert_eq!(
            grid_track_to_template_component(&GridTrack::Px(5.0)),
            GridTemplateComponent::Single(sizing(length(5.0_f32), length(5.0_f32)))
        );
    }

    #[test]
    fn line_placement_maps_start_and_end_separately() {
        let placement = GridLinePlacement::new(GridPlacement::Line(2), GridPlacement::Span(3));
        let converted = grid_line_placement_to_taffy(placement);
        assert_eq!(converted.start, line(2));
        assert_eq!(converted.end, span(3));
        assert_eq!(
            grid_placement_to_taffy(GridPlacement::Auto),
            taffy::style::GridPlacement::Auto
        );
    }

    #[test]
    fn auto_flow_maps_direction_and_density() {
        let cases = [
            (GridAutoFlow::Row, taffy::style::GridAutoFlow::Row),
            (GridAutoFlow::Column, taffy::style::GridAutoFlow::Column),
            (GridAutoFlow::RowDense, taffy::style::GridAutoFlow::RowDense),
            (
                GridAutoFlow::ColumnDense,
                taffy::style::GridAutoFlow::ColumnDense,
            ),
        ];
        for (flow, expected) in cases {
            assert_eq!(grid_auto_flow_to_taffy(flow), expected);
        }
    }
}
