use super::runner::EventLoop;
use crate::features::layout_engine::domain::{DisplayCommandSender, FloatingKind, PopupTarget};
use crate::features::module_runtime::domain::RenderOutcome;
use crate::features::module_runtime::ports::{LayoutEvent, LayoutEventSender};
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::render::{ModuleFrame, PlacedBuffer};
use crate::shared::primitives::{LayoutSurface, ModuleId, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use crate::shared::wayland::ports::SurfaceParent;
use std::collections::{HashMap, HashSet};

/// Whether a module actor is embedded in a parent's tree or stands on its own.
#[derive(Debug, Clone, Copy)]
pub(super) enum Embedding<'a> {
    /// The bar itself, or any module with no parent: it covers every monitor.
    TopLevel,
    /// A module some parent embeds, present on a monitor only while that parent gives it a site.
    Embedded {
        /// Every monitor the actor has rendered for: one of them with no layout lost its site.
        seen: &'a HashSet<MonitorId>,
    },
}

/// Only ever consults which monitors have *some* entry, never a value — kept
/// generic so it isn't coupled to whatever a layout channel's value type is.
///
/// A top-level actor (`Embedding::TopLevel` — the bar itself, or any module
/// with no parent) has no "site" to lose: it always covers every monitor
/// Wayland currently reports, on every pass. Suspension (decision 14) only
/// makes sense for an *embedded* actor, whose presence on a monitor is a
/// fact some parent asserts by giving it a rect in `layouts` — for that
/// actor, `seen` (every monitor this actor has ever produced a render tree
/// for, i.e. `RenderPipeline::render_trees` keys, carried by
/// `Embedding::Embedded`) matters: a monitor already in `seen` but absent
/// from `layouts` has no live site any more (its embedding rect was
/// withdrawn) and must stay suspended, not be rediscovered via the
/// bootstrap path below. A monitor this embedded actor
/// never rendered before still needs that bootstrap path: it renders once,
/// unconstrained, purely to report an intrinsic size before its parent has
/// assigned it a rect.
///
/// Wayland's `monitor_scales` is the sole authority on whether a monitor
/// exists at all: it's populated the moment an output's `wl_output::Name`/
/// `Scale` event arrives and cleaned the moment its global is removed,
/// whereas Hyprland's own view can lag a hotplug by a beat (workspaces move
/// onto a new monitor before `monitoraddedv2` arrives) and `module_sizes` is
/// a cache that only ever gains entries — unioning either of those in, as
/// this used to, let a disconnected monitor stay "discovered" forever. A
/// `layouts` entry for a monitor Wayland doesn't (or no longer) know about
/// is likewise never a discovery source on its own — layout is a
/// consequence of a Wayland surface existing, not the other way round.
#[must_use]
pub(super) fn discover_monitors<V>(
    hub: &SignalHub,
    layouts: &HashMap<MonitorId, V>,
    embedding: Embedding<'_>,
) -> Vec<MonitorId> {
    let wayland_monitors: HashSet<MonitorId> =
        hub.monitor_scales_rx().borrow().keys().cloned().collect();
    let Embedding::Embedded { seen } = embedding else {
        return wayland_monitors.into_iter().collect();
    };
    let mut all_monitors: HashSet<MonitorId> = HashSet::new();
    for monitor in layouts.keys() {
        if wayland_monitors.contains(monitor) {
            all_monitors.insert(monitor.clone());
        }
    }
    for monitor in &wayland_monitors {
        if !seen.contains(monitor) {
            all_monitors.insert(monitor.clone());
        }
    }
    all_monitors.into_iter().collect()
}

/// Builds this child's `SurfaceParent` from its fixed embedding `surface`
/// (decision 5: identity, and so embedding site, is fixed for the actor's
/// life) plus the monitor being rendered — a popup/panel's `FloatingKind`
/// needs both `parent_id` and `monitor_id` to name the exact floating
/// surface this child lives inside.
pub(super) fn surface_parent_for(
    parent_id: ModuleId,
    surface: LayoutSurface,
    monitor_id: &MonitorId,
) -> SurfaceParent {
    match surface {
        LayoutSurface::Bar => SurfaceParent::Bar,
        LayoutSurface::Popup => SurfaceParent::Floating(FloatingKind::Popup(PopupTarget::new(
            parent_id,
            monitor_id.clone(),
        ))),
        LayoutSurface::Panel => SurfaceParent::Floating(FloatingKind::Panel(PopupTarget::new(
            parent_id,
            monitor_id.clone(),
        ))),
        LayoutSurface::Tooltip => SurfaceParent::Floating(FloatingKind::Tooltip),
    }
}

impl<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> EventLoop<F, LS, DS, US>
{
    pub(super) fn dispatch_outcome_layouts(
        &mut self,
        monitor_id: &MonitorId,
        outcome: &RenderOutcome,
    ) {
        let layout_sender = self.ctx.layout_sender();
        let module_id = self.ctx.id();
        let has_children_now = !outcome.child_layouts().is_empty();
        let had_children_before = self
            .child_layouts_present
            .get(monitor_id)
            .copied()
            .unwrap_or(false);

        // Sent whenever there's something to report, and once more — with an
        // empty list — the render where a parent's last child disappears (a
        // popup that just closed): `AppState` needs that transition to clear
        // the stale `computed_layouts` entry and suspend the child (decision
        // 14). A parent that never had children stays silent, exactly as
        // before.
        if has_children_now || had_children_before {
            layout_sender.send_layout_event(LayoutEvent::ContainerLayoutsCalculated {
                parent_id: module_id,
                monitor_id: monitor_id.clone(),
                layouts: outcome.child_layouts().to_vec(),
            });
        }
        self.child_layouts_present
            .insert(monitor_id.clone(), has_children_now);

        if let Some(size_change) = outcome.size_change() {
            layout_sender.send_layout_event(LayoutEvent::ModuleSizeChanged {
                monitor_id: monitor_id.clone(),
                module_id,
                size: size_change.new_size(),
                settled_width: size_change.settled_width(),
            });
        }
    }

    pub(super) fn dispatch_outcome_buffer(&self, monitor_id: &MonitorId, outcome: &RenderOutcome) {
        if let Some((buffer, position)) = outcome.buffer().cloned() {
            let surface = self.ctx.surface();
            let parent = self
                .ctx
                .parent_id()
                .map(|pid| surface_parent_for(pid, surface, monitor_id));
            self.ctx.surface_manager().submit_child_buffer(
                ModuleFrame::new(
                    self.ctx.id(),
                    monitor_id.clone(),
                    PlacedBuffer::new(position, buffer),
                ),
                parent,
            );
        }
    }
}
