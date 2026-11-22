use super::floating_anchor::AnchorInfo;
use super::state::WaylandState;
use wayland_client::QueueHandle;
use wayland_protocols::xdg::shell::client::xdg_positioner::{
    Anchor as XdgAnchor, ConstraintAdjustment, Gravity as XdgGravity, XdgPositioner,
};
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

pub(super) fn create_positioner(
    xdg_wm_base: &XdgWmBase,
    queue_handle: &QueueHandle<WaylandState>,
    text_w: i32,
    text_h: i32,
    anchor_info: &AnchorInfo,
    offset: crate::shared::primitives::PopupOffset,
) -> XdgPositioner {
    let positioner = xdg_wm_base.create_positioner(queue_handle, ());
    positioner.set_size(text_w.max(1), text_h.max(1));
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
    use crate::shared::wayland::adapters::wayland::floating_anchor::FloatingParentRole;
    use crate::shared::wayland::adapters::wayland::tests::fixture::Fake;
    use wayland_client::Proxy;

    const SET_SIZE: u16 = 1;
    const SET_ANCHOR_RECT: u16 = 2;
    const SET_OFFSET: u16 = 6;

    #[test]
    fn positioner_carries_the_logical_size_the_anchor_rect_and_the_offset() {
        let fake = Fake::new();
        let state = fake.state(PopupEmbed::Composite);
        let xdg_wm_base = state.xdg_wm_base.as_ref().expect("xdg_wm_base");
        let anchor_info = AnchorInfo {
            bar_scale: 1,
            parent_role: FloatingParentRole::Popup(fake.xdg_surface()),
            anchor_x: 3,
            anchor_y: 4,
            anchor_w: 0,
            anchor_h: 7,
            target_monitor_id: MonitorId::new("DP-1"),
        };
        fake.requests();

        let positioner = create_positioner(
            xdg_wm_base,
            &fake.queue_handle,
            0,
            5,
            &anchor_info,
            PopupOffset::new(-2, 9),
        );

        let object = positioner.id().protocol_id();
        let sent: Vec<(u16, [u32; 4])> = fake
            .requests()
            .into_iter()
            .filter(|request| request.object == object)
            .map(|request| (request.opcode, request.args))
            .collect();
        // Zero extents are raised to 1: a positioner rejects empty rects.
        assert!(sent.contains(&(SET_SIZE, [1, 5, 0, 0])), "{sent:?}");
        assert!(sent.contains(&(SET_ANCHOR_RECT, [3, 4, 1, 7])), "{sent:?}");
        let offset = [(-2_i32).cast_unsigned(), 9, 0, 0];
        assert!(sent.contains(&(SET_OFFSET, offset)), "{sent:?}");
    }
}
