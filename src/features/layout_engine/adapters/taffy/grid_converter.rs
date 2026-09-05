use crate::features::styling::domain::{
    GridAutoFlow, GridLinePlacement, GridPlacement, GridTrack,
};
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
pub fn grid_track_to_track_sizing_function(
    track: &GridTrack,
) -> taffy::style::TrackSizingFunction {
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
