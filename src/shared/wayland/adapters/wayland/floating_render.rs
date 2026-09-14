use super::floating_anchor::AnchorInfo;
use super::state::WaylandState;
use wayland_client::QueueHandle;
use wayland_protocols::xdg::shell::client::xdg_positioner::{
    Anchor as XdgAnchor, ConstraintAdjustment, Gravity as XdgGravity, XdgPositioner,
};
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

pub(crate) fn create_positioner(
    xdg_wm_base: &XdgWmBase,
    qh: &QueueHandle<WaylandState>,
    text_w: i32,
    text_h: i32,
    anchor_info: &AnchorInfo,
    offset: crate::shared::primitives::PopupOffset,
) -> XdgPositioner {
    let positioner = xdg_wm_base.create_positioner(qh, ());
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
