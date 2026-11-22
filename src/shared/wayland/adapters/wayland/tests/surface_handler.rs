//! `handle_cmd` on the bar and on modules' own subsurfaces: missing globals,
//! missing parents, and a module resubmitting at a new size and position.

use crate::shared::config::domain::PopupEmbed;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::render::{ModuleFrame, PlacedBuffer};
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::adapters::wayland::command::SurfaceCommand;
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use crate::shared::wayland::adapters::wayland::surface_handler::handle_cmd;
use crate::shared::wayland::adapters::wayland::tests::fixture::wire::{damage_of, id};
use crate::shared::wayland::adapters::wayland::tests::fixture::{
    Fake, lose_runtime_dir, popup_kind, solid,
};
use crate::shared::wayland::ports::DisplayServerError;
use crate::shared::wayland::surface_parent::SurfaceParent;

/// `wl_subsurface.set_position`.
const SET_POSITION: u16 = 1;

/// Module 2's frame `buffer` at `position` on DP-1, parented to `parent`.
fn cmd(parent: Option<SurfaceParent>, position: Position, buffer: RenderBuffer) -> SurfaceCommand {
    SurfaceCommand::new(
        ModuleFrame::new(
            ModuleId::new(2),
            MonitorId::new("DP-1"),
            PlacedBuffer::new(position, buffer),
        ),
        parent,
    )
}

fn handle(
    fake: &Fake,
    state: &mut WaylandState,
    cmd: &SurfaceCommand,
) -> Result<(), DisplayServerError> {
    handle_cmd(state, &fake.queue_handle, cmd)
}

/// A state with a bar on DP-1.
fn with_bar(fake: &Fake, embed: PopupEmbed) -> WaylandState {
    let mut state = fake.state(embed);
    state.bars.push(fake.bar("DP-1", 1));
    state
}

#[test]
fn frame_for_a_monitor_without_a_bar_is_dropped() {
    let fake = Fake::new();
    let mut state = fake.state(PopupEmbed::Composite);
    let frame = cmd(
        Some(SurfaceParent::Bar),
        Position::new(0, 0),
        solid(1, 1, [1; 4]),
    );
    handle(&fake, &mut state, &frame).expect("dropped");
    assert!(state.surface_to_id.is_empty());
}

#[test]
fn bar_resize_without_memory_fails() {
    let fake = Fake::new();
    let mut state = with_bar(&fake, PopupEmbed::Composite);
    lose_runtime_dir(&mut state);
    let frame = cmd(None, Position::new(0, 0), solid(3, 2, [1; 4]));
    assert!(handle(&fake, &mut state, &frame).is_err());
    let bar = &state.bars[0];
    assert_eq!((bar.shm_buffer.width(), bar.shm_buffer.height()), (1, 1));
}

#[test]
fn bar_frame_of_a_new_size_gets_a_buffer_of_that_size_and_damages_all_of_it() {
    let fake = Fake::new();
    let mut state = with_bar(&fake, PopupEmbed::Composite);
    fake.requests();
    let frame = cmd(None, Position::new(0, 0), solid(3, 2, [1; 4]));

    handle(&fake, &mut state, &frame).expect("bar frame");

    let bar = &state.bars[0];
    assert_eq!((bar.shm_buffer.width(), bar.shm_buffer.height()), (3, 2));
    assert_eq!(
        damage_of(&fake.requests(), &bar.surface),
        vec![[0, 0, 3, 2]]
    );
}

#[test]
fn frame_without_shm_is_dropped() {
    let fake = Fake::new();
    let mut state = with_bar(&fake, PopupEmbed::Composite);
    let kind = popup_kind();
    let mut floating = fake.floating(Size::new(2, 1), solid(2, 1, [5; 4]));
    floating.configure.mark_configured();
    state.floating_surfaces.insert(kind.clone(), floating);
    state.shm = None;

    let bar_frame = cmd(None, Position::new(0, 0), solid(3, 2, [1; 4]));
    handle(&fake, &mut state, &bar_frame).expect("bar frame dropped");
    let child = cmd(
        Some(SurfaceParent::Floating(kind.clone())),
        Position::new(1, 0),
        solid(1, 1, [1; 4]),
    );
    handle(&fake, &mut state, &child).expect("child dropped");

    assert_eq!(state.bars[0].shm_buffer.width(), 1);
    assert!(state.surface_to_id.is_empty());
    assert!(state.floating_surfaces[&kind].layers.is_empty());
    assert!(state.pending_children.is_empty());
}

#[test]
fn module_surface_without_its_globals_or_memory_fails() {
    let fake = Fake::new();
    let unbinds: [fn(&mut WaylandState); 3] = [
        |state| state.compositor = None,
        |state| state.subcompositor = None,
        lose_runtime_dir,
    ];
    for unbind in unbinds {
        let mut state = with_bar(&fake, PopupEmbed::Composite);
        unbind(&mut state);
        let frame = cmd(
            Some(SurfaceParent::Bar),
            Position::new(0, 0),
            solid(1, 1, [1; 4]),
        );
        assert!(handle(&fake, &mut state, &frame).is_err());
        assert!(state.bars[0].module_surfaces.is_empty());
    }
}

#[test]
fn child_of_a_floating_surface_not_shown_is_dropped() {
    let fake = Fake::new();
    let mut state = with_bar(&fake, PopupEmbed::Subsurface);
    let parent = SurfaceParent::Floating(popup_kind());
    let frame = cmd(Some(parent), Position::new(0, 0), solid(1, 1, [1; 4]));
    handle(&fake, &mut state, &frame).expect("dropped");
    assert!(state.surface_to_id.is_empty());
    assert!(state.bars[0].module_surfaces.is_empty());
}

#[test]
fn module_resubmitted_bigger_elsewhere_gets_a_new_buffer_and_moves() {
    let fake = Fake::new();
    let mut state = with_bar(&fake, PopupEmbed::Composite);
    let first = cmd(
        Some(SurfaceParent::Bar),
        Position::new(0, 0),
        solid(1, 1, [1; 4]),
    );
    handle(&fake, &mut state, &first).expect("first frame");
    fake.requests();

    let second = cmd(
        Some(SurfaceParent::Bar),
        Position::new(3, 4),
        solid(3, 2, [2; 4]),
    );
    handle(&fake, &mut state, &second).expect("second frame");

    let module = &state.bars[0].module_surfaces[&ModuleId::new(2)];
    assert_eq!(module.size, Size::new(3, 2));
    let shm_size = (module.shm_buffer.width(), module.shm_buffer.height());
    assert_eq!(shm_size, (3, 2));
    assert_eq!((module.x, module.y), (3, 4));
    let requests = fake.requests();
    let subsurface = id(&module.subsurface);
    let moves: Vec<[u32; 4]> = requests
        .iter()
        .filter(|request| request.object == subsurface && request.opcode == SET_POSITION)
        .map(|request| request.args)
        .collect();
    assert_eq!(moves, vec![[3, 4, 0, 0]]);
    assert_eq!(damage_of(&requests, &module.surface), vec![[0, 0, 3, 2]]);
    let registered = state
        .surface_to_id
        .get(&module.surface)
        .expect("registered");
    assert_eq!(registered.0, ModuleId::new(2));
}
