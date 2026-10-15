//! Pure pointer routing over composited child layers.

use super::CompositeLayers;
use crate::shared::events::core::SurfaceKind;
use crate::shared::primitives::{ModuleId, MonitorId, Position};

/// A pointer destination: the module, its monitor and the kind of surface.
pub type PointerTarget = (ModuleId, MonitorId, SurfaceKind);

/// The child layer under the pointer, with the position relative to that layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub module_id: ModuleId,
    pub monitor_id: MonitorId,
    pub local: Position,
}

/// Returns the topmost layer containing the logical position `pos`.
#[must_use]
pub fn hit(layers: &CompositeLayers, scale: i32, pos: Position) -> Option<Hit> {
    let divisor = u32::try_from(scale).unwrap_or(1).max(1);
    layers
        .iter()
        .filter_map(|layer| {
            let local = Position::new(
                pos.x().checked_sub(layer.origin().x())?,
                pos.y().checked_sub(layer.origin().y())?,
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
pub struct Crossing {
    pub leave: Option<PointerTarget>,
    pub enter: Option<PointerTarget>,
}

#[must_use]
pub fn crossing(prev: Option<&PointerTarget>, next: Option<&PointerTarget>) -> Crossing {
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
    use crate::shared::primitives::render::RenderBuffer;

    fn buf(w: u32, h: u32) -> RenderBuffer {
        let n = usize::try_from(w.saturating_mul(h).saturating_mul(4)).unwrap_or(0);
        RenderBuffer::new(vec![0u8; n], Size::new(w, h))
    }

    fn add(layers: &mut CompositeLayers, id: u32, origin: (i32, i32), size: (u32, u32)) {
        layers.upsert(
            ModuleId::new(id),
            MonitorId::new("m"),
            Position::new(origin.0, origin.1),
            buf(size.0, size.1),
        );
    }

    fn hit_id(layers: &CompositeLayers, scale: i32, x: i32, y: i32) -> Option<u32> {
        hit(layers, scale, Position::new(x, y)).map(|h| h.module_id.value())
    }

    #[test]
    fn hit_returns_none_outside_all_layers() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, (10, 10), (5, 5));
        assert_eq!(hit_id(&layers, 1, 0, 0), None);
        assert_eq!(hit_id(&layers, 1, 20, 20), None);
        assert_eq!(hit_id(&CompositeLayers::default(), 1, 0, 0), None);
    }

    #[test]
    fn hit_returns_topmost_of_overlapping() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, (0, 0), (10, 10));
        add(&mut layers, 2, (5, 5), (10, 10));
        assert_eq!(hit_id(&layers, 1, 6, 6), Some(2));
        assert_eq!(hit_id(&layers, 1, 1, 1), Some(1));
    }

    #[test]
    fn hit_local_is_child_relative() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, (10, 20), (10, 10));
        let h = hit(&layers, 1, Position::new(13, 25)).expect("hit");
        assert_eq!(h.local, Position::new(3, 5));
        assert_eq!(h.monitor_id, MonitorId::new("m"));
    }

    #[test]
    fn hit_respects_scale() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, (0, 0), (20, 20));
        assert_eq!(hit_id(&layers, 2, 9, 9), Some(1));
        assert_eq!(hit_id(&layers, 2, 10, 10), None);
    }

    #[test]
    fn hit_treats_non_positive_scale_as_one() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, (0, 0), (4, 4));
        assert_eq!(hit_id(&layers, 0, 3, 3), Some(1));
        assert_eq!(hit_id(&layers, -2, 4, 4), None);
    }

    #[test]
    fn hit_edge_is_exclusive_on_right_and_bottom() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, (2, 2), (4, 4));
        assert_eq!(hit_id(&layers, 1, 2, 2), Some(1));
        assert_eq!(hit_id(&layers, 1, 5, 5), Some(1));
        assert_eq!(hit_id(&layers, 1, 6, 5), None);
        assert_eq!(hit_id(&layers, 1, 5, 6), None);
        assert_eq!(hit_id(&layers, 1, 1, 2), None);
        assert_eq!(hit_id(&layers, 1, 2, 1), None);
    }

    fn target(id: u32) -> PointerTarget {
        (ModuleId::new(id), MonitorId::new("m"), SurfaceKind::Popup)
    }

    #[test]
    fn crossing_cases() {
        let (a, b) = (target(1), target(2));
        let cases: Vec<(Option<&PointerTarget>, Option<&PointerTarget>, Crossing)> = vec![
            (
                None,
                Some(&a),
                Crossing {
                    leave: None,
                    enter: Some(a.clone()),
                },
            ),
            (
                Some(&a),
                Some(&a),
                Crossing {
                    leave: None,
                    enter: None,
                },
            ),
            (
                Some(&a),
                Some(&b),
                Crossing {
                    leave: Some(a.clone()),
                    enter: Some(b.clone()),
                },
            ),
            (
                Some(&a),
                None,
                Crossing {
                    leave: Some(a.clone()),
                    enter: None,
                },
            ),
            (
                None,
                None,
                Crossing {
                    leave: None,
                    enter: None,
                },
            ),
        ];
        for (prev, next, expected) in cases {
            assert_eq!(crossing(prev, next), expected);
        }
    }
}
