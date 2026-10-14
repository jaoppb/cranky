//! Composite mode: embedded children's pixels blended into a floating
//! surface's own shm buffer instead of living on their own subsurfaces.

use super::floating_bounds::to_i32;
use super::floating_update::commit_surface;
use super::types::FloatingSurface;
use crate::shared::wayland::composite::composite_into;

/// Blends `floating.base` plus every child layer into the shm back buffer,
/// then commits it.
pub(super) fn repaint(floating: &mut FloatingSurface, bar_scale: i32) {
    let size = floating.size;
    composite_into(
        floating.shm_buffer.mmap_mut(),
        size,
        floating.base.data(),
        &floating.layers,
        bar_scale,
    );
    commit_surface(
        floating,
        bar_scale,
        to_i32(size.width()),
        to_i32(size.height()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::primitives::{ModuleId, MonitorId, PopupOffset, Position, Size};
    use crate::shared::wayland::adapters::wayland::floating_anchor::{
        AnchorInfo, FloatingParentRole,
    };
    use crate::shared::wayland::adapters::wayland::floating_setup::{
        FloatingPlacement, create_new_floating, update_existing_floating,
    };
    use crate::shared::wayland::adapters::wayland::tests::fixture::{
        Fake, committed_bytes, popup_kind, solid,
    };
    use crate::shared::wayland::composite::CompositeLayers;

    const BASE: [u8; 4] = [5, 5, 5, 255];
    const CHILD: [u8; 4] = [1, 2, 3, 255];
    /// A 2×1 base with the child's 1×1 layer blended over its right pixel.
    const BLENDED: [u8; 8] = [5, 5, 5, 255, 1, 2, 3, 255];

    fn child_layers() -> CompositeLayers {
        let mut layers = CompositeLayers::default();
        layers.upsert(
            ModuleId::new(9),
            MonitorId::new("DP-1"),
            Position::new(1, 0),
            solid(1, 1, CHILD),
        );
        layers
    }

    fn anchor(fake: &Fake) -> AnchorInfo {
        AnchorInfo {
            bar_scale: 1,
            parent_role: FloatingParentRole::Popup(fake.xdg_surface()),
            anchor_x: 0,
            anchor_y: 0,
            anchor_w: 1,
            anchor_h: 1,
            target_monitor_id: MonitorId::new("DP-1"),
        }
    }

    fn placement(size: Size) -> FloatingPlacement {
        FloatingPlacement {
            size,
            text_w: 2,
            text_h: 1,
            offset: PopupOffset::default(),
        }
    }

    #[test]
    fn repaint_blends_layers_over_base() {
        let fake = Fake::new();
        let mut floating = fake.floating(Size::new(2, 1), solid(2, 1, BASE));
        floating.layers = child_layers();

        repaint(&mut floating, 1);

        assert_eq!(committed_bytes(&mut floating.shm_buffer), BLENDED.to_vec());
    }

    #[test]
    fn create_new_floating_applies_pending_children() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let kind = popup_kind();
        state.pending_children.insert(kind.clone(), child_layers());
        let base = solid(2, 1, BASE);

        create_new_floating(
            &mut state,
            &fake.qh,
            kind.clone(),
            &base,
            &anchor(&fake),
            &placement(Size::new(2, 1)),
            None,
        )
        .expect("floating created");

        assert!(!state.pending_children.contains_key(&kind));
        let floating = state.floating_surfaces.get_mut(&kind).expect("floating");
        assert!(floating.layers.get(ModuleId::new(9)).is_some());
        assert_eq!(floating.base, base);
        // Not configured yet, so nothing is committed: the blend waits in the
        // back buffer the xdg_surface configure handler attaches.
        assert_eq!(floating.shm_buffer.mmap_mut().to_vec(), BLENDED.to_vec());
    }

    #[test]
    fn update_existing_floating_keeps_layers_when_size_changes() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let kind = popup_kind();
        let mut floating = fake.floating(Size::new(1, 1), solid(1, 1, [0, 0, 0, 255]));
        floating.layers = child_layers();
        state.floating_surfaces.insert(kind.clone(), floating);
        let base = solid(2, 1, BASE);

        update_existing_floating(
            &mut state,
            &fake.qh,
            &kind,
            &base,
            &anchor(&fake),
            &placement(Size::new(2, 1)),
        )
        .expect("floating updated");

        let floating = state.floating_surfaces.get_mut(&kind).expect("floating");
        assert_eq!(floating.size, Size::new(2, 1));
        assert!(floating.layers.get(ModuleId::new(9)).is_some());
        assert_eq!(floating.base, base);
        assert_eq!(committed_bytes(&mut floating.shm_buffer), BLENDED.to_vec());
    }
}
