//! Pure pointer routing over composited child layers.

use super::CompositeLayers;
use crate::shared::events::core::SurfaceKind;
use crate::shared::primitives::{ModuleId, MonitorId, Position};

/// A pointer destination: the module, its monitor and the kind of surface.
pub(in crate::shared::wayland) type PointerTarget = (ModuleId, MonitorId, SurfaceKind);

/// The child layer under the pointer, with the position relative to that layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::shared::wayland) struct Hit {
    pub(crate) module_id: ModuleId,
    pub(crate) monitor_id: MonitorId,
    pub(crate) local: Position,
}

/// Returns the topmost layer containing the logical position `pos`.
#[must_use]
pub(in crate::shared::wayland) fn hit(
    layers: &CompositeLayers,
    scale: i32,
    pos: Position,
) -> Option<Hit> {
    let divisor = u32::try_from(scale).unwrap_or(1).max(1);
    layers
        .iter()
        .filter_map(|layer| {
            let local = Position::new(
                pos.x().checked_sub(layer.position().x())?,
                pos.y().checked_sub(layer.position().y())?,
            );
            let width = layer.buffer().width().checked_div(divisor)?;
            let height = layer.buffer().height().checked_div(divisor)?;
            let inside = (0..i64::from(width)).contains(&i64::from(local.x()))
                && (0..i64::from(height)).contains(&i64::from(local.y()));
            inside.then(|| Hit {
                module_id: layer.module_id(),
                monitor_id: layer.monitor_id().clone(),
                local,
            })
        })
        .last()
}

/// The enter/leave pair produced by moving the pointer between targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::shared::wayland) struct Crossing {
    pub(crate) leave: Option<PointerTarget>,
    pub(crate) enter: Option<PointerTarget>,
}

#[must_use]
pub(in crate::shared::wayland) fn crossing(
    prev: Option<&PointerTarget>,
    next: Option<&PointerTarget>,
) -> Crossing {
    if prev == next {
        return Crossing {
            leave: None,
            enter: None,
        };
    }
    Crossing {
        leave: prev.cloned(),
        enter: next.cloned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::primitives::Size;
    use crate::shared::primitives::geometry::Rect;
    use crate::shared::primitives::render::{ModuleFrame, PlacedBuffer, RenderBuffer};

    fn buf(width: u32, height: u32) -> RenderBuffer {
        let count = usize::try_from(width.saturating_mul(height).saturating_mul(4)).unwrap_or(0);
        RenderBuffer::new(vec![0u8; count], Size::new(width, height))
    }

    /// Adds module `id`'s layer covering `bounds`.
    fn add(layers: &mut CompositeLayers, id: u32, bounds: Rect) {
        layers.upsert(ModuleFrame::new(
            ModuleId::new(id),
            MonitorId::new("m"),
            PlacedBuffer::new(
                Position::new(bounds.x(), bounds.y()),
                buf(bounds.width(), bounds.height()),
            ),
        ));
    }

    fn rect(origin: (i32, i32), size: (u32, u32)) -> Rect {
        Rect::new(Position::new(origin.0, origin.1), Size::new(size.0, size.1))
    }

    fn hit_id(layers: &CompositeLayers, scale: i32, pos: Position) -> Option<u32> {
        hit(layers, scale, pos).map(|layer_hit| layer_hit.module_id.value())
    }

    #[test]
    fn hit_returns_none_outside_all_layers() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, rect((10, 10), (5, 5)));
        assert_eq!(hit_id(&layers, 1, Position::new(0, 0)), None);
        assert_eq!(hit_id(&layers, 1, Position::new(20, 20)), None);
        assert_eq!(
            hit_id(&CompositeLayers::default(), 1, Position::new(0, 0)),
            None
        );
    }

    #[test]
    fn hit_returns_topmost_of_overlapping() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, rect((0, 0), (10, 10)));
        add(&mut layers, 2, rect((5, 5), (10, 10)));
        assert_eq!(hit_id(&layers, 1, Position::new(6, 6)), Some(2));
        assert_eq!(hit_id(&layers, 1, Position::new(1, 1)), Some(1));
    }

    #[test]
    fn hit_local_is_child_relative() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, rect((10, 20), (10, 10)));
        let layer_hit = hit(&layers, 1, Position::new(13, 25)).expect("hit");
        assert_eq!(layer_hit.local, Position::new(3, 5));
        assert_eq!(layer_hit.monitor_id, MonitorId::new("m"));
    }

    #[test]
    fn hit_respects_scale() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, rect((0, 0), (20, 20)));
        assert_eq!(hit_id(&layers, 2, Position::new(9, 9)), Some(1));
        assert_eq!(hit_id(&layers, 2, Position::new(10, 10)), None);
    }

    #[test]
    fn hit_treats_non_positive_scale_as_one() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, rect((0, 0), (4, 4)));
        assert_eq!(hit_id(&layers, 0, Position::new(3, 3)), Some(1));
        assert_eq!(hit_id(&layers, -2, Position::new(4, 4)), None);
    }

    #[test]
    fn hit_edge_is_exclusive_on_right_and_bottom() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, rect((2, 2), (4, 4)));
        assert_eq!(hit_id(&layers, 1, Position::new(2, 2)), Some(1));
        assert_eq!(hit_id(&layers, 1, Position::new(5, 5)), Some(1));
        assert_eq!(hit_id(&layers, 1, Position::new(6, 5)), None);
        assert_eq!(hit_id(&layers, 1, Position::new(5, 6)), None);
        assert_eq!(hit_id(&layers, 1, Position::new(1, 2)), None);
        assert_eq!(hit_id(&layers, 1, Position::new(2, 1)), None);
    }

    fn target(id: u32) -> PointerTarget {
        (ModuleId::new(id), MonitorId::new("m"), SurfaceKind::Popup)
    }

    fn crossing_of(leave: Option<&PointerTarget>, enter: Option<&PointerTarget>) -> Crossing {
        Crossing {
            leave: leave.cloned(),
            enter: enter.cloned(),
        }
    }

    #[test]
    fn crossing_cases() {
        let (first, second) = (target(1), target(2));
        let cases: Vec<(Option<&PointerTarget>, Option<&PointerTarget>, Crossing)> = vec![
            (None, Some(&first), crossing_of(None, Some(&first))),
            (Some(&first), Some(&first), crossing_of(None, None)),
            (
                Some(&first),
                Some(&second),
                crossing_of(Some(&first), Some(&second)),
            ),
            (Some(&first), None, crossing_of(Some(&first), None)),
            (None, None, crossing_of(None, None)),
        ];
        for (prev, next, expected) in cases {
            assert_eq!(crossing(prev, next), expected);
        }
    }
}
