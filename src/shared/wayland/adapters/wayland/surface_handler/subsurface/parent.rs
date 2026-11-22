//! Where a module frame's subsurface goes: its parent surface and that
//! parent's module surfaces.
use super::super::super::state::WaylandState;
use super::super::super::types::ModuleSurface;
use super::SubsurfaceFrame;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::events::core::SurfaceKind;
use crate::shared::primitives::ModuleId;
use crate::shared::wayland::surface_parent::SurfaceParent;
use std::collections::HashMap;
use wayland_client::protocol::wl_surface::WlSurface;

impl SubsurfaceFrame<'_> {
    /// The module surfaces of the frame's parent: its bar's, or the floating
    /// surface's it is embedded in; `None` while that surface does not exist.
    pub(super) fn module_surfaces_mut<'s>(
        &self,
        state: &'s mut WaylandState,
    ) -> Option<&'s mut HashMap<ModuleId, ModuleSurface>> {
        match self.parent {
            SurfaceParent::Bar => state
                .bars
                .get_mut(self.bar_index)
                .map(|bar| &mut bar.module_surfaces),
            SurfaceParent::Floating(kind) => state
                .floating_surfaces
                .get_mut(kind)
                .map(|floating| &mut floating.module_surfaces),
        }
    }

    /// The surface the frame's subsurface should be parented to, plus the
    /// `SurfaceKind` to record for it in `surface_to_id` — `None` for a
    /// tooltip child, which (like the tooltip's own surface in
    /// `setup_new_floating`) gets no pointer routing today. `None` overall
    /// means the parent isn't ready yet (a popup not yet shown, or already
    /// torn down).
    ///
    /// The kind is always `SurfaceKind::Bar` when present: it names which of
    /// the *child's* trees to hit-test, and a child's own subsurface always
    /// shows its main tree, whatever it happens to be parented to. Recording
    /// the parent's kind instead would hit-test the child's own (empty) popup
    /// tree, so clicks inside an embedded module would never land.
    pub(in super::super) fn parent_surface(
        &self,
        state: &WaylandState,
    ) -> Option<(WlSurface, Option<SurfaceKind>)> {
        match self.parent {
            SurfaceParent::Bar => state
                .bars
                .get(self.bar_index)
                .map(|bar| (bar.surface.clone(), Some(SurfaceKind::Bar))),
            SurfaceParent::Floating(kind) => {
                let surface_kind = match kind {
                    FloatingKind::Popup(_) | FloatingKind::Panel(_) => Some(SurfaceKind::Bar),
                    FloatingKind::Tooltip => None,
                };
                state
                    .floating_surfaces
                    .get(kind)
                    .map(|floating| (floating.surface.clone(), surface_kind))
            }
        }
    }
}
