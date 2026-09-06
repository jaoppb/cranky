use super::state::WaylandState;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::primitives::geometry::Rect;
use crate::shared::primitives::MonitorId;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;

pub(crate) struct AnchorInfo {
    pub bar_scale: i32,
    pub bar_layer_surface: ZwlrLayerSurfaceV1,
    pub anchor_x: i32,
    pub anchor_y: i32,
    pub anchor_w: i32,
    pub anchor_h: i32,
    pub target_monitor_id: MonitorId,
}

pub(crate) fn handle_conflicts(
    state: &mut WaylandState,
    target: &crate::features::layout_engine::domain::PopupTarget,
) {
    let behavior = state.hub.config_rx().borrow().popup().behavior();
    let active_targets: Vec<crate::features::layout_engine::domain::PopupTarget> = state
        .floating_surfaces
        .keys()
        .filter_map(|k| match k {
            FloatingKind::Popup(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    let conflicts = crate::features::layout_engine::domain::PopupExclusivity::compute_conflicts(
        &active_targets,
        target,
        behavior,
    );
    for conf in conflicts {
        let conf_kind = FloatingKind::Popup(conf.clone());
        if let Some(floating) = state.floating_surfaces.remove(&conf_kind) {
            state.surface_to_id.remove(&floating.surface);
            if conf.module_id() != target.module_id() || conf.monitor_id() != target.monitor_id() {
                let _ = state.hub.pointer_tx().send((
                    conf.module_id(),
                    conf.monitor_id().clone(),
                    crate::shared::events::core::PointerEvent::PopupDismissed,
                ));
            }
        }
    }
}

#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
pub(crate) fn resolve_anchor(
    state: &WaylandState,
    kind: &FloatingKind,
    monitor_id: Option<MonitorId>,
    anchor_rect: Option<Rect>,
) -> Option<AnchorInfo> {
    let mut bar_scale = 1;
    let mut bar_height = 0;
    let mut bar_layer_surface = None;
    let mut anchor_x = 0;
    let mut anchor_y = 0;
    let mut anchor_w = 1;
    let mut anchor_h = 1;
    let mut target_monitor_id = monitor_id.unwrap_or_else(|| MonitorId::new(""));

    match kind {
        FloatingKind::Tooltip => {
            let parent_surface = state.pointer_surface.clone()?;
            let mut pointer_x = state.pointer_pos.0;
            for bar in &state.bars {
                if bar.surface == parent_surface {
                    bar_scale = bar.scale;
                    bar_height = bar.height;
                    bar_layer_surface = Some(bar.layer_surface.clone());
                    target_monitor_id = MonitorId::new(&bar.output_name);
                    break;
                }
                if let Some(ms) = bar
                    .module_surfaces
                    .values()
                    .find(|m| m.surface == parent_surface)
                {
                    bar_scale = bar.scale;
                    bar_height = bar.height;
                    bar_layer_surface = Some(bar.layer_surface.clone());
                    pointer_x += f64::from(ms.x);
                    target_monitor_id = MonitorId::new(&bar.output_name);
                    break;
                }
            }
            anchor_x = pointer_x as i32;
            anchor_y = bar_height as i32;
        }
        FloatingKind::Popup(target) => {
            let module_id = target.module_id();
            let mon_name = target.monitor_id().as_str();
            for bar in &state.bars {
                if bar.output_name == mon_name {
                    bar_scale = bar.scale;
                    bar_layer_surface = Some(bar.layer_surface.clone());
                    target_monitor_id = target.monitor_id().clone();
                    if let Some(ms) = bar.module_surfaces.get(&module_id) {
                        if let Some(r) = anchor_rect {
                            anchor_x = ms.x + r.x();
                            anchor_y = ms.y + r.y();
                            anchor_w = r.width() as i32;
                            anchor_h = r.height() as i32;
                        } else {
                            anchor_x = ms.x;
                            anchor_y = ms.y;
                            anchor_w = ms.size.width() as i32;
                            anchor_h = ms.size.height() as i32;
                        }
                    } else if let Some(r) = anchor_rect {
                        anchor_x = r.x();
                        anchor_y = r.y();
                        anchor_w = r.width() as i32;
                        anchor_h = r.height() as i32;
                    }
                    break;
                }
            }
        }
    }

    let bar_layer_surface = bar_layer_surface?;
    Some(AnchorInfo {
        bar_scale,
        bar_layer_surface,
        anchor_x,
        anchor_y,
        anchor_w,
        anchor_h,
        target_monitor_id,
    })
}
