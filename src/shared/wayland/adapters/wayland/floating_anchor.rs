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
            let _ = state.hub.pointer_tx().send((
                conf.module_id(),
                conf.monitor_id().clone(),
                crate::shared::events::core::InteractionEvent::Lifecycle(
                    crate::shared::events::core::SurfaceLifecycleEvent::PopupDismissed,
                ),
            ));
        }
    }
}

fn to_i32(val: u32) -> i32 {
    i32::try_from(val).unwrap_or(i32::MAX)
}

struct ModuleAnchorBounds {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

fn compute_module_bounds(
    ms: Option<&super::types::ModuleSurface>,
    anchor_rect: Option<Rect>,
    default_h: i32,
    is_panel: bool,
) -> ModuleAnchorBounds {
    match (ms, anchor_rect) {
        (Some(ms), Some(r)) => {
            let y = if is_panel { ms.y.saturating_add(r.y()) } else { 0 };
            let h = if is_panel { to_i32(r.height()) } else { default_h };
            ModuleAnchorBounds {
                x: ms.x.saturating_add(r.x()),
                y,
                w: to_i32(r.width()),
                h,
            }
        }
        (Some(ms), None) => {
            let y = if is_panel { ms.y } else { 0 };
            let h = if is_panel { to_i32(ms.size.height()) } else { default_h };
            ModuleAnchorBounds {
                x: ms.x,
                y,
                w: to_i32(ms.size.width()),
                h,
            }
        }
        (None, Some(r)) => {
            let y = if is_panel { r.y() } else { 0 };
            let h = if is_panel { to_i32(r.height()) } else { default_h };
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
            anchor_x = i32::try_from(crate::utils::f64_to_i64(pointer_x)).unwrap_or(0);
            anchor_y = to_i32(bar_height);
        }
        FloatingKind::Popup(target) | FloatingKind::Panel(target) => {
            let is_panel = matches!(kind, FloatingKind::Panel(_));
            let module_id = target.module_id();
            let mon_name = target.monitor_id().as_str();
            for bar in &state.bars {
                if bar.output_name == mon_name {
                    bar_scale = bar.scale;
                    bar_layer_surface = Some(bar.layer_surface.clone());
                    target_monitor_id = target.monitor_id().clone();
                    let bounds = compute_module_bounds(
                        bar.module_surfaces.get(&module_id),
                        anchor_rect,
                        to_i32(bar.height),
                        is_panel,
                    );
                    anchor_x = bounds.x;
                    anchor_y = bounds.y;
                    anchor_w = bounds.w;
                    anchor_h = bounds.h;
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
