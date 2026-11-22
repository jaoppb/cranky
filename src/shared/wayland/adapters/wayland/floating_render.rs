use super::floating_frame::FloatingPlacement;
use super::state::WaylandState;
use wayland_client::QueueHandle;
use wayland_protocols::xdg::shell::client::xdg_positioner::{
    Anchor as XdgAnchor, ConstraintAdjustment, Gravity as XdgGravity, XdgPositioner,
};
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

/// A positioner for `placement`: its logical size, below its anchor rect,
/// shifted by its offset.
pub(super) fn create_positioner(
    xdg_wm_base: &XdgWmBase,
    queue_handle: &QueueHandle<WaylandState>,
    placement: &FloatingPlacement,
) -> XdgPositioner {
    let (anchor_info, offset) = (&placement.anchor, placement.offset);
    let positioner = xdg_wm_base.create_positioner(queue_handle, ());
    positioner.set_size(placement.text_w.max(1), placement.text_h.max(1));
    positioner.set_anchor_rect(
        anchor_info.anchor_x,
        anchor_info.anchor_y,
        anchor_info.anchor_w.max(1),
        anchor_info.anchor_h.max(1),
    );
    positioner.set_anchor(XdgAnchor::Bottom);
    positioner.set_gravity(XdgGravity::Bottom);
    positioner.set_offset(offset.dx(), offset.dy());
    positioner.set_constraint_adjustment(
        ConstraintAdjustment::SlideX | ConstraintAdjustment::SlideY | ConstraintAdjustment::FlipY,
    );
    positioner
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::primitives::{MonitorId, PopupOffset};
    use crate::shared::wayland::adapters::wayland::floating_anchor::{
        AnchorInfo, FloatingParentRole,
    };
    use crate::shared::wayland::adapters::wayland::tests::fixture::Fake;
    use wayland_client::Proxy;

    const SET_SIZE: u16 = 1;
    const SET_ANCHOR_RECT: u16 = 2;
    const SET_OFFSET: u16 = 6;

    /// The positioner of a 0×5 placement at (3, 4), 0×7, shifted by
    /// (-2, 9), and every request sent to it as `(opcode, first words)`.
    fn positioned(fake: &Fake) -> Vec<(u16, [u32; 4])> {
        let state = fake.state(PopupEmbed::Composite);
        let xdg_wm_base = state.xdg_wm_base.as_ref().expect("xdg_wm_base");
        let anchor = AnchorInfo {
            bar_scale: 1,
            parent_role: FloatingParentRole::Popup(fake.xdg_surface()),
            anchor_x: 3,
            anchor_y: 4,
            anchor_w: 0,
            anchor_h: 7,
            target_monitor_id: MonitorId::new("DP-1"),
        };
        fake.requests();
        let placement = FloatingPlacement {
            anchor,
            text_w: 0,
            text_h: 5,
            offset: PopupOffset::new(-2, 9),
        };
        let positioner = create_positioner(xdg_wm_base, &fake.queue_handle, &placement);
        let object = positioner.id().protocol_id();
        fake.requests()
            .into_iter()
            .filter(|request| request.object == object)
            .map(|request| (request.opcode, request.args))
            .collect()
    }

    #[test]
    fn positioner_carries_the_logical_size_the_anchor_rect_and_the_offset() {
        let fake = Fake::new();
        let sent = positioned(&fake);
        // Zero extents are raised to 1: a positioner rejects empty rects.
        assert!(sent.contains(&(SET_SIZE, [1, 5, 0, 0])), "{sent:?}");
        assert!(sent.contains(&(SET_ANCHOR_RECT, [3, 4, 1, 7])), "{sent:?}");
        let offset = [(-2_i32).cast_unsigned(), 9, 0, 0];
        assert!(sent.contains(&(SET_OFFSET, offset)), "{sent:?}");
    }
}
