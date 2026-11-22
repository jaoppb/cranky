use super::floating_anchor::{AnchorInfo, FloatingParentRole};
use super::floating_render::create_positioner;
use super::input_region::{is_click_through, set_empty_input_region};
use super::state::WaylandState;
use super::types::FloatingSurface;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::events::core::SurfaceKind;
use crate::shared::primitives::PopupOffset;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::composite::PixelsMut;
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;
use wayland_protocols::xdg::shell::client::xdg_positioner::XdgPositioner;

pub(super) use super::floating_update::update_existing_floating;

/// Everything Wayland needs to place a floating surface — no rendering
/// information, since the module already painted `buffer`. `size` is the
/// buffer's own physical (scaled) size; `text_w`/`text_h` are the logical
/// size the positioner works in.
pub(super) struct FloatingPlacement {
    pub(super) size: Size,
    pub(super) text_w: i32,
    pub(super) text_h: i32,
    pub(super) offset: PopupOffset,
}

struct NewFloatingInfo {
    shm_buffer: ShmBuffer,
    size: Size,
    positioner: XdgPositioner,
    base: RenderBuffer,
}

pub(super) fn create_new_floating(
    state: &mut WaylandState,
    queue_handle: &QueueHandle<WaylandState>,
    kind: FloatingKind,
    buffer: &RenderBuffer,
    anchor_info: &AnchorInfo,
    placement: &FloatingPlacement,
    parent: Option<FloatingKind>,
) -> Result<(), DisplayServerError> {
    let shm = state
        .shm
        .as_ref()
        .ok_or_else(|| DisplayServerError::ConnectionFailed {
            reason: "SHM not bound".to_string(),
        })?;
    let shm_buffer = ShmBuffer::new(
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

    setup_new_floating(
        state,
        queue_handle,
        kind,
        anchor_info,
        NewFloatingInfo {
            shm_buffer,
            size: placement.size,
            positioner,
            base: buffer.clone(),
        },
        parent,
    )
}

fn setup_new_floating(
    state: &mut WaylandState,
    queue_handle: &QueueHandle<WaylandState>,
    kind: FloatingKind,
    anchor_info: &AnchorInfo,
    mut info: NewFloatingInfo,
    parent: Option<FloatingKind>,
) -> Result<(), DisplayServerError> {
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
    let (size, scale) = (info.size, anchor_info.bar_scale);
    let mut pixels = PixelsMut::new(info.shm_buffer.mmap_mut(), size);
    pixels.copy_from(info.base.data());
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
    let xdg_popup = xdg_surface.get_popup(parent_xdg_surface, &info.positioner, queue_handle, ());

    if let FloatingKind::Popup(_) = &kind
        && let (Some(seat), Some(serial)) = (state.seat.as_ref(), state.last_button_serial)
    {
        xdg_popup.grab(seat, serial.value());
    }

    if let FloatingParentRole::Layer(layer) = &anchor_info.parent_role {
        layer.get_popup(&xdg_popup);
    }
    info.positioner.destroy();
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
            shm_buffer: info.shm_buffer,
            size: info.size,
            reposition_token: 0,
            module_surfaces: std::collections::HashMap::new(),
            parent,
            base: info.base,
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
    use crate::shared::primitives::MonitorId;
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
            size: Size::new(2, 1),
            text_w: 2,
            text_h: 1,
            offset: PopupOffset::default(),
        };
        create_new_floating(
            state,
            &fake.queue_handle,
            popup_kind(),
            &solid(2, 1, [1; 4]),
            &anchor(fake),
            &placement,
            parent,
        )
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
