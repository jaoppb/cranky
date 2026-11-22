use super::floating_anchor::FloatingParentRole;
use super::floating_render::create_positioner;
use super::input_region::{is_click_through, set_empty_input_region};
use super::state::WaylandState;
use super::types::FloatingSurface;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::events::core::SurfaceKind;
use crate::shared::wayland::adapters::shm::{ShmBuffer, ShmSource};
use crate::shared::wayland::composite::PixelsMut;
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;
use wayland_protocols::xdg::shell::client::xdg_positioner::XdgPositioner;

pub(super) use super::floating_frame::{FloatingFrame, FloatingPlacement};
pub(super) use super::floating_update::update_existing_floating;

/// A new floating surface's frame with the Wayland objects already
/// allocated for it.
struct NewFloatingInfo {
    frame: FloatingFrame,
    shm_buffer: ShmBuffer,
    positioner: XdgPositioner,
}

pub(super) fn create_new_floating(
    state: &mut WaylandState,
    queue_handle: &QueueHandle<WaylandState>,
    frame: FloatingFrame,
) -> Result<(), DisplayServerError> {
    let shm = state
        .shm
        .as_ref()
        .ok_or_else(|| DisplayServerError::ConnectionFailed {
            reason: "SHM not bound".to_string(),
        })?;
    let shm_buffer = ShmBuffer::new(
        ShmSource::new(shm, state.app_env.xdg_runtime_dir().as_path()),
        *frame.buffer.size(),
        queue_handle,
    )
    .map_err(|error| DisplayServerError::Internal(error.to_string()))?;

    let xdg_wm_base =
        state
            .xdg_wm_base
            .as_ref()
            .ok_or_else(|| DisplayServerError::ConnectionFailed {
                reason: "XDG WM Base not bound".to_string(),
            })?;
    let positioner = create_positioner(xdg_wm_base, queue_handle, &frame.placement);

    setup_new_floating(
        state,
        queue_handle,
        NewFloatingInfo {
            frame,
            shm_buffer,
            positioner,
        },
    )
}

fn setup_new_floating(
    state: &mut WaylandState,
    queue_handle: &QueueHandle<WaylandState>,
    info: NewFloatingInfo,
) -> Result<(), DisplayServerError> {
    let NewFloatingInfo {
        frame,
        mut shm_buffer,
        positioner,
    } = info;
    let FloatingFrame {
        kind,
        parent,
        buffer: base,
        placement,
    } = frame;
    let anchor_info = &placement.anchor;
    let xdg_wm_base =
        state
            .xdg_wm_base
            .as_ref()
            .ok_or_else(|| DisplayServerError::ConnectionFailed {
                reason: "XDG WM Base not bound".to_string(),
            })?;
    let compositor =
        state
            .compositor
            .as_ref()
            .ok_or_else(|| DisplayServerError::ConnectionFailed {
                reason: "Compositor not bound".to_string(),
            })?;
    // Past every fallible step: claim children that rendered before this
    // surface existed. Blend only; the configure handler attaches it.
    let layers = state.pending_children.remove(&kind).unwrap_or_default();
    let (size, scale) = (*base.size(), anchor_info.bar_scale);
    let mut pixels = PixelsMut::new(shm_buffer.mmap_mut(), size);
    pixels.copy_from(base.data());
    pixels.blend_layers(&layers, scale);
    let surface = compositor.create_surface(queue_handle, ());
    surface.set_buffer_scale(anchor_info.bar_scale);
    if is_click_through(&kind) {
        set_empty_input_region(compositor, &surface, queue_handle);
    }
    let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, queue_handle, ());
    // A popup nested inside another floating surface (decision 7) names its
    // parent directly; a top-level one leaves it unset and relies on
    // `zwlr_layer_surface_v1.get_popup` below instead.
    let parent_xdg_surface = match &anchor_info.parent_role {
        FloatingParentRole::Popup(parent_surface) => Some(parent_surface),
        FloatingParentRole::Layer(_) => None,
    };
    let xdg_popup = xdg_surface.get_popup(parent_xdg_surface, &positioner, queue_handle, ());

    if let FloatingKind::Popup(_) = &kind
        && let (Some(seat), Some(serial)) = (state.seat.as_ref(), state.last_button_serial)
    {
        xdg_popup.grab(seat, serial.value());
    }

    if let FloatingParentRole::Layer(layer) = &anchor_info.parent_role {
        layer.get_popup(&xdg_popup);
    }
    positioner.destroy();
    surface.commit();

    match &kind {
        FloatingKind::Popup(target) => {
            state.surface_to_id.insert(
                surface.clone(),
                (
                    target.module_id(),
                    anchor_info.target_monitor_id.clone(),
                    SurfaceKind::Popup,
                ),
            );
        }
        FloatingKind::Panel(target) => {
            state.surface_to_id.insert(
                surface.clone(),
                (
                    target.module_id(),
                    anchor_info.target_monitor_id.clone(),
                    SurfaceKind::Panel,
                ),
            );
        }
        FloatingKind::Tooltip => {}
    }

    state.floating_surfaces.insert(
        kind,
        FloatingSurface {
            surface,
            xdg_surface,
            xdg_popup,
            shm_buffer,
            size,
            reposition_token: 0,
            module_surfaces: std::collections::HashMap::new(),
            parent,
            base,
            layers,
            configure: super::types::ConfigureGate::default(),
            scale: anchor_info.bar_scale,
        },
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::events::core::PointerSerial;
    use crate::shared::primitives::{MonitorId, PopupOffset};
    use crate::shared::wayland::adapters::wayland::floating_anchor::AnchorInfo;
    use crate::shared::wayland::adapters::wayland::state::WaylandState;
    use crate::shared::wayland::adapters::wayland::tests::fixture::wire::id;
    use crate::shared::wayland::adapters::wayland::tests::fixture::{
        Fake, lose_runtime_dir, popup_kind, solid,
    };

    /// `xdg_popup.grab`.
    const GRAB: u16 = 1;

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

    fn create(
        fake: &Fake,
        state: &mut WaylandState,
        parent: Option<FloatingKind>,
    ) -> Result<(), DisplayServerError> {
        let placement = FloatingPlacement {
            anchor: anchor(fake),
            text_w: 2,
            text_h: 1,
            offset: PopupOffset::default(),
        };
        let frame = FloatingFrame {
            kind: popup_kind(),
            parent,
            buffer: solid(2, 1, [1; 4]),
            placement,
        };
        create_new_floating(state, &fake.queue_handle, frame)
    }

    #[test]
    fn create_new_floating_without_shm_or_its_memory_fails() {
        let fake = Fake::new();
        let unbinds: [fn(&mut WaylandState); 2] = [|state| state.shm = None, lose_runtime_dir];
        for unbind in unbinds {
            let mut state = fake.state(PopupEmbed::Composite);
            unbind(&mut state);
            assert!(create(&fake, &mut state, None).is_err());
            assert!(state.floating_surfaces.is_empty());
        }
    }

    #[test]
    fn create_new_floating_records_its_parent() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let parent = FloatingKind::Tooltip;
        create(&fake, &mut state, Some(parent.clone())).expect("created");
        assert_eq!(state.floating_surfaces[&popup_kind()].parent, Some(parent));
    }

    #[test]
    fn new_popup_grabs_the_seat_with_the_last_button_serial() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let seat = fake.seat();
        state.seat = Some(seat.clone());
        state.last_button_serial = Some(PointerSerial::new(42));
        fake.requests();

        create(&fake, &mut state, None).expect("created");

        let popup = id(&state.floating_surfaces[&popup_kind()].xdg_popup);
        let grabs: Vec<[u32; 4]> = fake
            .requests()
            .into_iter()
            .filter(|request| request.object == popup && request.opcode == GRAB)
            .map(|request| request.args)
            .collect();
        assert_eq!(grabs, vec![[id(&seat), 42, 0, 0]]);
    }
}
