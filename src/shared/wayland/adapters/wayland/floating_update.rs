use super::floating_anchor::AnchorInfo;
use super::floating_composite::repaint;
use super::floating_render::create_positioner;
use super::floating_setup::FloatingPlacement;
use super::state::WaylandState;
use super::types::FloatingSurface;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;

pub(super) fn commit_surface(
    floating: &mut FloatingSurface,
    bar_scale: i32,
    w_i32: i32,
    h_i32: i32,
) {
    floating.surface.set_buffer_scale(bar_scale);
    floating
        .surface
        .attach(Some(floating.shm_buffer.current_buffer()), 0, 0);
    floating.surface.damage_buffer(0, 0, w_i32, h_i32);
    floating.surface.commit();
    floating.shm_buffer.swap_buffers();
}

pub(super) fn update_existing_floating(
    state: &mut WaylandState,
    queue_handle: &QueueHandle<WaylandState>,
    kind: &FloatingKind,
    buffer: &RenderBuffer,
    anchor_info: &AnchorInfo,
    placement: &FloatingPlacement,
) -> Result<(), DisplayServerError> {
    let Some(floating) = state.floating_surfaces.get_mut(kind) else {
        return Ok(());
    };
    floating.scale = anchor_info.bar_scale;

    if floating.size == placement.size {
        floating.base = buffer.clone();
        repaint(floating, anchor_info.bar_scale);
        return Ok(());
    }

    let shm = state
        .shm
        .as_ref()
        .ok_or_else(|| DisplayServerError::ConnectionFailed {
            reason: "SHM not bound".to_string(),
        })?;
    let new_shm_buffer = ShmBuffer::new(
        shm,
        placement.size.width(),
        placement.size.height(),
        queue_handle,
        state.app_env.xdg_runtime_dir().as_path(),
    )
    .map_err(|error| DisplayServerError::Internal(error.to_string()))?;

    let xdg_wm_base =
        state
            .xdg_wm_base
            .as_ref()
            .ok_or_else(|| DisplayServerError::ConnectionFailed {
                reason: "XDG WM Base not bound".to_string(),
            })?;
    let positioner = create_positioner(
        xdg_wm_base,
        queue_handle,
        placement.text_w,
        placement.text_h,
        anchor_info,
        placement.offset,
    );
    floating.reposition_token = floating.reposition_token.wrapping_add(1);
    floating
        .xdg_popup
        .reposition(&positioner, floating.reposition_token);
    positioner.destroy();
    // Blend into the new buffer, not the old one, before committing it.
    floating.shm_buffer = new_shm_buffer;
    floating.size = placement.size;
    // Only now: `base` must keep the shm buffer's size if any step above fails.
    floating.base = buffer.clone();
    repaint(floating, anchor_info.bar_scale);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::PopupTarget;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::primitives::geometry::Size;
    use crate::shared::primitives::{ModuleId, MonitorId, PopupOffset};
    use crate::shared::wayland::adapters::wayland::floating_anchor::FloatingParentRole;
    use crate::shared::wayland::adapters::wayland::tests::fixture::wire::damage_of;
    use crate::shared::wayland::adapters::wayland::tests::fixture::{
        Fake, lose_runtime_dir, popup_kind, solid,
    };

    /// Shows a 2×1 frame as `kind` over whatever `state` holds.
    fn update(
        fake: &Fake,
        state: &mut WaylandState,
        kind: &FloatingKind,
    ) -> Result<(), DisplayServerError> {
        let anchor = AnchorInfo {
            bar_scale: 1,
            parent_role: FloatingParentRole::Popup(fake.xdg_surface()),
            anchor_x: 0,
            anchor_y: 0,
            anchor_w: 1,
            anchor_h: 1,
            target_monitor_id: MonitorId::new("DP-1"),
        };
        let placement = FloatingPlacement {
            size: Size::new(2, 1),
            text_w: 2,
            text_h: 1,
            offset: PopupOffset::default(),
        };
        let base = solid(2, 1, [1; 4]);
        update_existing_floating(state, &fake.queue_handle, kind, &base, &anchor, &placement)
    }

    #[test]
    fn update_of_a_surface_not_shown_does_nothing() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let other = FloatingKind::Panel(PopupTarget::new(ModuleId::new(5), MonitorId::new("DP-1")));
        update(&fake, &mut state, &other).expect("ignored");
        assert!(state.floating_surfaces.is_empty());
    }

    #[test]
    fn failed_resize_keeps_the_old_buffer() {
        let fake = Fake::new();
        let unbinds: [fn(&mut WaylandState); 2] = [lose_runtime_dir, |state| {
            state.xdg_wm_base = None;
        }];
        for unbind in unbinds {
            let mut state = fake.state(PopupEmbed::Composite);
            let floating = fake.floating(Size::new(1, 1), solid(1, 1, [0; 4]));
            state.floating_surfaces.insert(popup_kind(), floating);
            unbind(&mut state);

            assert!(update(&fake, &mut state, &popup_kind()).is_err());

            let floating = &state.floating_surfaces[&popup_kind()];
            assert_eq!(floating.size, Size::new(1, 1));
            assert_eq!(*floating.base.size(), Size::new(1, 1));
        }
    }

    #[test]
    fn repaint_after_configure_damages_the_whole_surface() {
        let fake = Fake::new();
        let mut floating = fake.floating(Size::new(2, 1), solid(2, 1, [1; 4]));
        floating.configure.mark_configured();
        fake.requests();

        repaint(&mut floating, 1);

        let damage = damage_of(&fake.requests(), &floating.surface);
        assert_eq!(damage, vec![[0, 0, 2, 1]]);
    }

    #[test]
    fn resize_allocates_and_damages_the_new_size() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let mut floating = fake.floating(Size::new(1, 1), solid(1, 1, [0; 4]));
        floating.configure.mark_configured();
        state.floating_surfaces.insert(popup_kind(), floating);
        fake.requests();

        update(&fake, &mut state, &popup_kind()).expect("resized");

        let floating = &state.floating_surfaces[&popup_kind()];
        let shm_size = (floating.shm_buffer.width(), floating.shm_buffer.height());
        assert_eq!((floating.size, shm_size), (Size::new(2, 1), (2, 1)));
        let damage = damage_of(&fake.requests(), &floating.surface);
        assert_eq!(damage, vec![[0, 0, 2, 1]]);
    }
}
