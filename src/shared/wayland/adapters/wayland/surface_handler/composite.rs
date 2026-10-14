//! Composite-mode routing of an embedded child's frame into its floating
//! parent's layers.

use super::super::command::SurfaceCommand;
use super::super::floating_composite::repaint;
use super::super::state::WaylandState;
use crate::features::layout_engine::domain::FloatingKind;

/// Records `cmd`'s pixels as a layer of the floating surface `kind` and
/// repaints it, or holds them pending until that surface exists. The layer's
/// origin is the child's logical position; scale comes from the monitor's bar.
/// No `surface_to_id` entry: the child has no surface of its own.
pub(super) fn handle_composite_child(
    state: &mut WaylandState,
    bar_index: usize,
    kind: &FloatingKind,
    cmd: &SurfaceCommand,
) {
    let bar_scale = state.bars.get(bar_index).map_or(1, |bar| bar.scale);
    let (module_id, monitor_id) = (cmd.module_id(), cmd.monitor_id().clone());
    let (origin, buffer) = (cmd.position(), cmd.buffer().clone());
    if let Some(floating) = state.floating_surfaces.get_mut(kind) {
        floating
            .layers
            .upsert(module_id, monitor_id, origin, buffer);
        repaint(floating, bar_scale);
    } else {
        state
            .pending_children
            .entry(kind.clone())
            .or_default()
            .upsert(module_id, monitor_id, origin, buffer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::primitives::{ModuleId, MonitorId, Position, Size};
    use crate::shared::wayland::adapters::wayland::surface_handler::handle_cmd;
    use crate::shared::wayland::adapters::wayland::tests::fixture::{
        Fake, committed_bytes, popup_kind, solid,
    };
    use crate::shared::wayland::surface_parent::SurfaceParent;

    fn child_cmd(kind: &FloatingKind, origin: Position) -> SurfaceCommand {
        SurfaceCommand::new(
            ModuleId::new(9),
            Some(SurfaceParent::Floating(kind.clone())),
            MonitorId::new("DP-1"),
            origin,
            solid(1, 1, [1, 2, 3, 255]),
        )
    }

    /// A state with a bar on DP-1 and an open 2×1 popup over a grey base.
    fn with_open_popup(fake: &Fake, embed: PopupEmbed) -> (WaylandState, FloatingKind) {
        let mut state = fake.state(embed);
        state.bars.push(fake.bar("DP-1", 1));
        let kind = popup_kind();
        let mut floating = fake.floating(Size::new(2, 1), solid(2, 1, [5, 5, 5, 255]));
        floating.configure.mark_configured();
        state.floating_surfaces.insert(kind.clone(), floating);
        (state, kind)
    }

    const BLENDED: [u8; 8] = [5, 5, 5, 255, 1, 2, 3, 255];

    #[test]
    fn child_before_configure_is_attached_blended_by_configure() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        state.bars.push(fake.bar("DP-1", 1));
        let kind = popup_kind();
        let floating = fake.floating(Size::new(2, 1), solid(2, 1, [5, 5, 5, 255]));
        let back = floating.shm_buffer.current_buffer().clone();
        state.floating_surfaces.insert(kind.clone(), floating);

        handle_composite_child(&mut state, 0, &kind, &child_cmd(&kind, Position::new(1, 0)));

        let floating = state.floating_surfaces.get_mut(&kind).expect("floating");
        assert_eq!(
            floating.shm_buffer.current_buffer(),
            &back,
            "no commit/swap"
        );
        assert_eq!(floating.shm_buffer.mmap_mut().to_vec(), BLENDED.to_vec());
        fake.configure(&mut state, &kind);
        let floating = state.floating_surfaces.get_mut(&kind).expect("floating");
        assert!(floating.configure.is_configured());
        assert_eq!(committed_bytes(&mut floating.shm_buffer), BLENDED.to_vec());
    }

    #[test]
    fn child_origin_is_scaled_by_bar_scale() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        state.bars.push(fake.bar("DP-1", 2));
        let kind = popup_kind();
        let mut floating = fake.floating(Size::new(4, 2), solid(4, 2, [5, 5, 5, 255]));
        floating.configure.mark_configured();
        state.floating_surfaces.insert(kind.clone(), floating);

        handle_cmd(&mut state, &fake.qh, &child_cmd(&kind, Position::new(1, 0))).expect("handled");

        let floating = state.floating_surfaces.get_mut(&kind).expect("floating");
        let mut expected = [5u8, 5, 5, 255].repeat(8);
        // Logical x=1 at scale 2 is physical x=2: bytes 8..12 of row 0.
        expected[8..12].copy_from_slice(&[1, 2, 3, 255]);
        assert_eq!(committed_bytes(&mut floating.shm_buffer), expected);
    }

    #[test]
    fn child_before_popup_is_held_pending() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let kind = popup_kind();

        handle_composite_child(&mut state, 0, &kind, &child_cmd(&kind, Position::new(3, 4)));

        assert!(state.floating_surfaces.is_empty());
        let layer = state
            .pending_children
            .get(&kind)
            .and_then(|layers| layers.get(ModuleId::new(9)))
            .expect("pending layer");
        assert_eq!(*layer.origin(), Position::new(3, 4));
    }

    #[test]
    fn child_with_open_popup_is_layered_and_repainted() {
        let fake = Fake::new();
        let (mut state, kind) = with_open_popup(&fake, PopupEmbed::Composite);

        handle_composite_child(&mut state, 0, &kind, &child_cmd(&kind, Position::new(1, 0)));

        assert!(state.pending_children.is_empty());
        assert!(state.surface_to_id.is_empty());
        let floating = state.floating_surfaces.get_mut(&kind).expect("floating");
        assert!(floating.layers.get(ModuleId::new(9)).is_some());
        assert_eq!(
            committed_bytes(&mut floating.shm_buffer),
            vec![5, 5, 5, 255, 1, 2, 3, 255]
        );
    }

    #[test]
    fn composite_mode_routes_floating_child_to_layers() {
        let fake = Fake::new();
        let (mut state, kind) = with_open_popup(&fake, PopupEmbed::Composite);

        handle_cmd(&mut state, &fake.qh, &child_cmd(&kind, Position::new(1, 0))).expect("handled");

        let floating = state.floating_surfaces.get(&kind).expect("floating");
        assert!(floating.layers.get(ModuleId::new(9)).is_some());
        assert!(floating.module_surfaces.is_empty());
    }

    #[test]
    fn subsurface_mode_uses_old_path() {
        let fake = Fake::new();
        let (mut state, kind) = with_open_popup(&fake, PopupEmbed::Subsurface);

        handle_cmd(&mut state, &fake.qh, &child_cmd(&kind, Position::new(1, 0))).expect("handled");

        let floating = state.floating_surfaces.get(&kind).expect("floating");
        assert!(floating.layers.is_empty());
        assert!(floating.module_surfaces.contains_key(&ModuleId::new(9)));
    }
}
