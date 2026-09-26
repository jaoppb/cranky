use crate::shared::primitives::geometry::Rect;

pub(super) fn to_i32(val: u32) -> i32 {
    i32::try_from(val).unwrap_or(i32::MAX)
}

pub(super) struct ModuleAnchorBounds {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
}

/// `precise` requests the exact rect (used by panels, and by any popup
/// nested inside another floating surface — decision 7's "translate a
/// child's anchor rect from its subsurface coordinates into the parent
/// popup's space") over the bar-popup convention of a fixed-height column
/// spanning the whole bar regardless of click position.
pub(super) fn compute_module_bounds(
    ms: Option<&super::types::ModuleSurface>,
    anchor_rect: Option<Rect>,
    default_h: i32,
    precise: bool,
) -> ModuleAnchorBounds {
    match (ms, anchor_rect) {
        (Some(ms), Some(r)) => {
            let y = if precise { ms.y.saturating_add(r.y()) } else { 0 };
            let h = if precise { to_i32(r.height()) } else { default_h };
            ModuleAnchorBounds {
                x: ms.x.saturating_add(r.x()),
                y,
                w: to_i32(r.width()),
                h,
            }
        }
        (Some(ms), None) => {
            let y = if precise { ms.y } else { 0 };
            let h = if precise { to_i32(ms.size.height()) } else { default_h };
            ModuleAnchorBounds {
                x: ms.x,
                y,
                w: to_i32(ms.size.width()),
                h,
            }
        }
        (None, Some(r)) => {
            let y = if precise { r.y() } else { 0 };
            let h = if precise { to_i32(r.height()) } else { default_h };
            ModuleAnchorBounds {
                x: r.x(),
                y,
                w: to_i32(r.width()),
                h,
            }
        }
        (None, None) => ModuleAnchorBounds {
            x: 0,
            y: 0,
            w: 1,
            h: default_h,
        },
    }
}
