use super::floating_anchor::{AnchorInfo, FloatingParentRole};
use super::floating_bounds::to_i32;
use super::state::WaylandState;
use super::types::{FloatingSurface, WaylandBar};
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::primitives::MonitorId;
use wayland_client::protocol::wl_surface::WlSurface;

/// Finds the bar or floating surface that owns the hovered element and
/// derives a tooltip's anchor from it. Tries every bar first (a module
/// embedded directly in the bar tree, or the bar itself), then every
/// currently open floating surface's own `module_surfaces` — structurally
/// parallel to a bar's — for a module embedded inside an open popup/panel
/// (`ui.module()` nested inside floating content). Without the second
/// pass, hovering such a module would leave the tooltip anchor unresolved.
/// A child composited into the popup's own surface is found through
/// `pointer_target` instead; hovering the popup owner's own content still
/// resolves nothing. A tooltip anchors on one point of its parent.
pub(super) fn resolve_tooltip_anchor(state: &WaylandState) -> Option<AnchorInfo> {
    let parent_surface = state.pointer_surface.as_ref()?;
    let parent = state
        .bars
        .iter()
        .find_map(|bar| bar_tooltip_parent(bar, parent_surface))
        .or_else(|| {
            state
                .floating_surfaces
                .iter()
                .find_map(|entry| floating_tooltip_parent(state, entry, parent_surface))
        })?;
    let pointer_x = state.pointer_pos.0 + f64::from(parent.child_x);
    Some(AnchorInfo {
        bar_scale: parent.bar_scale,
        parent_role: parent.role,
        anchor_x: i32::try_from(crate::utils::f64_to_i64(pointer_x)).unwrap_or(0),
        anchor_y: to_i32(parent.bar_height),
        anchor_w: 1,
        anchor_h: 1,
        target_monitor_id: parent.monitor_id,
    })
}

/// The surface a tooltip nests in, found from the hovered surface: its role,
/// monitor and bar, and the hovered surface's x origin on it.
struct TooltipParent {
    role: FloatingParentRole,
    monitor_id: MonitorId,
    bar_scale: i32,
    bar_height: u32,
    child_x: i32,
}

/// `bar` as the tooltip's parent when `hovered` is the bar itself or one of
/// its module subsurfaces.
fn bar_tooltip_parent(bar: &WaylandBar, hovered: &WlSurface) -> Option<TooltipParent> {
    let child_x = if bar.surface == *hovered {
        0
    } else {
        bar.module_surfaces
            .values()
            .find(|child| child.surface == *hovered)?
            .x
    };
    Some(TooltipParent {
        role: FloatingParentRole::Layer(bar.layer_surface.clone()),
        monitor_id: MonitorId::new(&bar.output_name),
        bar_scale: bar.scale,
        bar_height: bar.height,
        child_x,
    })
}

/// The popup or panel of `entry` as the tooltip's parent when `hovered` is
/// a child composited into it or one of its module subsurfaces; a tooltip is
/// never a parent. Without a bar on its monitor, scale 1 and height 0.
fn floating_tooltip_parent(
    state: &WaylandState,
    entry: (&FloatingKind, &FloatingSurface),
    hovered: &WlSurface,
) -> Option<TooltipParent> {
    let (FloatingKind::Popup(target) | FloatingKind::Panel(target), floating) = entry else {
        return None;
    };
    // A composited child's pointer coordinates are already local to
    // the popup surface, so it adds no origin; a subsurface child's
    // are local to its own surface, so it adds its offset.
    let child_x = if floating.surface == *hovered && hovers_layer(state, floating) {
        0
    } else {
        floating
            .module_surfaces
            .values()
            .find(|child| child.surface == *hovered)?
            .x
    };
    let monitor_id = target.monitor_id();
    let bar = state
        .bars
        .iter()
        .find(|bar| bar.output_name == monitor_id.as_str());
    Some(TooltipParent {
        role: FloatingParentRole::Popup(floating.xdg_surface.clone()),
        monitor_id: monitor_id.clone(),
        bar_scale: bar.map_or(1, |bar| bar.scale),
        bar_height: bar.map_or(0, |bar| bar.height),
        child_x,
    })
}

/// Whether the pointer target is a child composited into `floating`, rather
/// than the floating surface's own owner.
fn hovers_layer(state: &WaylandState, floating: &FloatingSurface) -> bool {
    state
        .pointer_target
        .as_ref()
        .is_some_and(|(id, monitor, _)| {
            floating
                .layers
                .get(*id)
                .is_some_and(|layer| layer.monitor_id() == monitor)
        })
}

#[cfg(test)]
mod tests {
    use super::resolve_tooltip_anchor;
    use crate::features::layout_engine::domain::FloatingKind;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::primitives::geometry::Size;
    use crate::shared::primitives::{ModuleId, MonitorId};
    use crate::shared::wayland::adapters::wayland::floating_anchor::FloatingParentRole;
    use crate::shared::wayland::adapters::wayland::state::WaylandState;
    use crate::shared::wayland::adapters::wayland::tests::fixture::{Fake, solid};

    /// A state with a bar 24 high at scale 2 on DP-1 and the pointer at x 12.
    fn with_bar(fake: &Fake) -> WaylandState {
        let mut state = fake.state(PopupEmbed::Composite);
        let mut bar = fake.bar("DP-1", 2);
        bar.height = 24;
        state.bars.push(bar);
        state.pointer_pos = (12.0, 4.0);
        state
    }

    #[test]
    fn no_pointer_surface_anchors_nothing() {
        let fake = Fake::new();
        let state = with_bar(&fake);
        assert!(resolve_tooltip_anchor(&state).is_none());
    }

    /// Hovering the bar itself anchors under the pointer, below the bar.
    #[test]
    fn bar_surface_anchors_on_its_layer_surface() {
        let fake = Fake::new();
        let mut state = with_bar(&fake);
        state.pointer_surface = Some(state.bars[0].surface.clone());
        let anchor = resolve_tooltip_anchor(&state).expect("anchor");
        let layer = state.bars[0].layer_surface.clone();
        assert!(
            matches!(anchor.parent_role, FloatingParentRole::Layer(ref found) if *found == layer)
        );
        assert_eq!(anchor.target_monitor_id, MonitorId::new("DP-1"));
        let placed = (anchor.bar_scale, anchor.anchor_x, anchor.anchor_y);
        assert_eq!(placed, (2, 12, 24));
    }

    /// Hovering a bar module's subsurface adds the module's offset.
    #[test]
    fn bar_module_anchors_at_its_offset_on_the_bar() {
        let fake = Fake::new();
        let mut state = with_bar(&fake);
        let mut module = fake.module_surface(&state.bars[0].surface);
        module.x = 5;
        state.pointer_surface = Some(module.surface.clone());
        state.bars[0]
            .module_surfaces
            .insert(ModuleId::new(2), module);
        let anchor = resolve_tooltip_anchor(&state).expect("anchor");
        assert!(matches!(anchor.parent_role, FloatingParentRole::Layer(_)));
        let placed = (anchor.bar_scale, anchor.anchor_x, anchor.anchor_y);
        assert_eq!(placed, (2, 17, 24));
    }

    /// A module embedded in an open tooltip is never a tooltip's parent.
    #[test]
    fn module_inside_a_tooltip_anchors_nothing() {
        let fake = Fake::new();
        let mut state = with_bar(&fake);
        let mut tooltip = fake.floating(Size::new(1, 1), solid(1, 1, [0; 4]));
        let module = fake.module_surface(&tooltip.surface);
        state.pointer_surface = Some(module.surface.clone());
        tooltip.module_surfaces.insert(ModuleId::new(2), module);
        state
            .floating_surfaces
            .insert(FloatingKind::Tooltip, tooltip);
        assert!(resolve_tooltip_anchor(&state).is_none());
    }
}
