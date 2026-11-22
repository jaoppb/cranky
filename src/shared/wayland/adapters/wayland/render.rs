use super::state::WaylandState;
use crate::features::module_runtime::ports::LayoutSender;
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{ChildBounds, ModuleId, MonitorId, SizeConstraint};
use crate::shared::wayland::ports::AppReadModel;
use std::collections::HashMap;
use tracing::{debug, info_span};

pub(super) fn render_outputs<Snd: LayoutSender>(
    state: &mut WaylandState,
    read_model: &AppReadModel,
    layout_senders: &HashMap<ModuleId, Snd>,
) {
    let span = info_span!("render_all_outputs");
    let _enter = span.enter();

    if state.bars.is_empty() {
        debug!("No bars available for rendering.");
        return;
    }

    let mut all_layouts_by_module: HashMap<ModuleId, HashMap<MonitorId, ChildBounds>> =
        HashMap::new();

    for bar in &mut state.bars {
        if !bar.configure.is_configured() {
            debug!("Skipping render for unconfigured bar: {}", bar.output_name);
            continue;
        }
        let width = bar.width;

        let is_focused = state
            .hub
            .hyprland_rx()
            .borrow()
            .focused_monitor()
            .is_some_and(|name| name.as_str() == bar.output_name);

        let mut root_config = read_model.config().root().clone();
        if !is_focused {
            root_config = root_config.as_unfocused();
        }

        // Check if hot-reload of height or margin is needed
        if bar.config_height != root_config.height().value()
            || bar.config_margin != *root_config.margin()
        {
            debug!(
                "Hot-reloading bar height/margin for output: {}",
                bar.output_name
            );
            bar.config_height = root_config.height().value();
            bar.config_margin = *root_config.margin();

            let margin = root_config.margin();
            bar.layer_surface.set_size(0, bar.config_height);
            bar.layer_surface.set_margin(
                margin.top().value(),
                margin.right().value(),
                margin.bottom().value(),
                margin.left().value(),
            );
            let height_i32 = i32::try_from(bar.config_height).unwrap_or(0);
            let zone = height_i32
                .saturating_add(margin.top().value())
                .saturating_add(margin.bottom().value());
            bar.layer_surface.set_exclusive_zone(zone);
            bar.surface.commit();
        }

        let monitor_id = MonitorId::new(&bar.output_name);
        if let Some(root_id) = read_model.root_module() {
            let bar_rect = Rect::new(Position::new(0, 0), Size::new(width, bar.config_height));
            // The root always gets a full, two-axis pin matching its own
            // layer-surface bounds — it has no parent to leave an axis free
            // for, so this reproduces exactly the sizing it always had.
            let constraint = SizeConstraint::new(Some(width), Some(bar.config_height));
            all_layouts_by_module
                .entry(root_id)
                .or_default()
                .insert(monitor_id, ChildBounds::new(bar_rect, constraint));
        }
    }

    // Broadcast root layout bounds to the root module for ALL active monitors
    for (id, sender) in layout_senders {
        if let Some(rects) = all_layouts_by_module.get(id) {
            sender.send_layout(rects.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::render_outputs;
    use crate::features::module_runtime::ports::LayoutSender;
    use crate::features::workspaces::domain::MonitorName;
    use crate::shared::config::domain::{
        AxisMargins, Config, CreatePartialRootConfigCommand, MarginConfig, MarginOffset,
        PartialRootConfig, PopupEmbed, RootConfig,
    };
    use crate::shared::events::signals::HyprlandState;
    use crate::shared::primitives::geometry::{BarHeight, Position, Rect, Size};
    use crate::shared::primitives::{ChildBounds, ModuleId, MonitorId, SizeConstraint};
    use crate::shared::wayland::adapters::wayland::adapter::{WaylandAdapter, readiness_socket};
    use crate::shared::wayland::adapters::wayland::state::WaylandState;
    use crate::shared::wayland::adapters::wayland::tests::fixture::Fake;
    use crate::shared::wayland::adapters::wayland::tests::fixture::wire::id;
    use crate::shared::wayland::adapters::wayland::types::{ConfigureGate, WaylandBar};
    use crate::shared::wayland::ports::AppReadModel;
    use crate::shared::wayland::ports::DisplayServerPort;
    use std::collections::BTreeMap;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::Mutex;
    use wayland_client::Proxy;

    /// `zwlr_layer_surface_v1.set_margin`.
    const SET_MARGIN: u16 = 3;

    /// Records every layout it is sent.
    #[derive(Default)]
    struct RecordingSender {
        sent: Mutex<Vec<HashMap<MonitorId, ChildBounds>>>,
    }

    impl LayoutSender for RecordingSender {
        fn send_layout(&self, layout: HashMap<MonitorId, ChildBounds>) {
            self.sent.lock().unwrap().push(layout);
        }
    }

    /// Renders one bar on DP-1, configured or not, and returns how many
    /// layouts the root received and how many requests reached the bar's
    /// surfaces.
    fn render_bar(configure: ConfigureGate) -> (usize, usize) {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let mut bar = fake.bar("DP-1", 1);
        bar.configure = configure;
        let bar_objects = [
            bar.surface.id().protocol_id(),
            bar.layer_surface.id().protocol_id(),
        ];
        state.bars.push(bar);
        let root = ModuleId::new(1);
        let mut read_model = AppReadModel::new(Config::default(), Some(root));
        read_model.module_ids = vec![root];
        let senders = HashMap::from([(root, RecordingSender::default())]);
        fake.requests();
        render_outputs(&mut state, &read_model, &senders);
        let to_bar = fake
            .requests()
            .iter()
            .filter(|request| bar_objects.contains(&request.object))
            .count();
        let layouts = senders[&root].sent.lock().unwrap().len();
        (layouts, to_bar)
    }

    /// `render_all` sends the bar's resize to the compositor itself: nothing
    /// else flushes the connection before the next event wait.
    #[test]
    fn render_all_flushes_the_bar_resize() {
        crate::test_utils::block_on(async {
            let fake = Fake::new();
            let mut state = fake.state(PopupEmbed::Composite);
            state.bars.push(fake.bar("DP-1", 1));
            let config_rx = state.hub.config_rx();
            let connection = fake.connection().clone();
            let (_notify_tx, notify_rx) = tokio::sync::mpsc::channel(1);
            let mut adapter = WaylandAdapter {
                socket: readiness_socket(&connection).expect("socket"),
                event_queue: connection.new_event_queue(),
                connection,
                state,
                pending_surfaces: Arc::new(Mutex::new(HashMap::new())),
                notify_rx,
                config_rx,
            };
            fake.requests();
            let read_model = AppReadModel::new(Config::default(), None);
            let senders: HashMap<ModuleId, RecordingSender> = HashMap::new();

            adapter.render_all(&read_model, &senders).expect("rendered");

            assert!(!fake.received().is_empty(), "nothing reached the socket");
        });
    }

    #[test]
    fn unconfigured_bar_is_skipped() {
        assert_eq!(render_bar(ConfigureGate::Pending), (0, 0));
        // The same bar, configured, is resized to the config height and laid out.
        let (layouts, to_bar) = render_bar(ConfigureGate::Configured);
        assert_eq!(layouts, 1);
        assert!(to_bar > 0);
    }

    /// The layouts the root module 1 receives from one render of `state`
    /// with `root` as the read model's root.
    fn root_layouts(
        state: &mut WaylandState,
        root: Option<ModuleId>,
    ) -> Vec<HashMap<MonitorId, ChildBounds>> {
        let module = ModuleId::new(1);
        let read_model = AppReadModel::new(Config::default().with_root(unfocused_root()), root);
        let senders = HashMap::from([(module, RecordingSender::default())]);
        render_outputs(state, &read_model, &senders);
        senders[&module].sent.lock().unwrap().clone()
    }

    /// How many layouts the root module 1 receives from one render of
    /// `state` with `root` as the read model's root.
    fn layouts_sent(state: &mut WaylandState, root: Option<ModuleId>) -> usize {
        root_layouts(state, root).len()
    }

    /// The root's bounds on a `width`×`height` bar: the whole bar, pinned on
    /// both axes.
    const fn full_bar_bounds(width: u32, height: u32) -> ChildBounds {
        let rect = Rect::new(Position::new(0, 0), Size::new(width, height));
        ChildBounds::new(rect, SizeConstraint::new(Some(width), Some(height)))
    }

    /// A configured bar on `name`, `width` wide, its surface 7 high: a height
    /// neither the root's nor its unfocused one.
    fn wide_bar(fake: &Fake, name: &str, width: u32) -> WaylandBar {
        let mut bar = fake.bar(name, 1);
        bar.width = width;
        bar.height = 7;
        bar
    }

    /// A root 30 high, 20 on an unfocused monitor.
    fn unfocused_root() -> RootConfig {
        let unfocused = PartialRootConfig::new(CreatePartialRootConfigCommand::new(
            Some(BarHeight::new(20)),
            None,
            None,
        ));
        RootConfig::default().with_unfocused(Some(unfocused))
    }

    fn focus(state: &WaylandState, monitor: &str) {
        let focused = Some(MonitorName::new(monitor));
        let hyprland = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), focused);
        state.hub.hyprland_tx().send(hyprland).expect("hyprland");
    }

    #[test]
    fn no_bar_sends_no_layout() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        assert_eq!(layouts_sent(&mut state, Some(ModuleId::new(1))), 0);
    }

    /// The focused monitor's bar takes the root height, any other bar the
    /// unfocused one, and the root lays out at each bar's width and config
    /// height, whatever its surface's height.
    #[test]
    fn bar_height_follows_the_monitor_focus() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        state.bars.push(wide_bar(&fake, "DP-1", 800));
        state.bars.push(wide_bar(&fake, "HDMI-A-1", 640));
        focus(&state, "DP-1");
        let layouts = root_layouts(&mut state, Some(ModuleId::new(1)));
        let heights: Vec<u32> = state.bars.iter().map(|bar| bar.config_height).collect();
        assert_eq!(heights, vec![30, 20]);
        let expected = HashMap::from([
            (MonitorId::new("DP-1"), full_bar_bounds(800, 30)),
            (MonitorId::new("HDMI-A-1"), full_bar_bounds(640, 20)),
        ]);
        assert_eq!(layouts, vec![expected]);
    }

    /// A margin change alone resizes the bar's layer surface.
    #[test]
    fn margin_change_alone_reloads_the_bar() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let mut bar = fake.bar("DP-1", 1);
        bar.config_height = 20;
        let offset = MarginOffset::new(3);
        let margins = AxisMargins::new(offset, offset);
        bar.config_margin = MarginConfig::new(margins, margins);
        let layer_surface = id(&bar.layer_surface);
        state.bars.push(bar);
        fake.requests();
        layouts_sent(&mut state, Some(ModuleId::new(1)));
        assert_eq!(state.bars[0].config_margin, MarginConfig::default());
        let margins_set: Vec<[u32; 4]> = fake
            .requests()
            .iter()
            .filter(|request| request.object == layer_surface && request.opcode == SET_MARGIN)
            .map(|request| request.args)
            .collect();
        assert_eq!(margins_set, vec![[0; 4]]);
    }

    /// Without a root module no layout is sent, but the bar is still resized.
    #[test]
    fn without_a_root_module_the_bar_is_resized_but_not_laid_out() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        state.bars.push(fake.bar("DP-1", 1));
        assert_eq!(layouts_sent(&mut state, None), 0);
        assert_eq!(state.bars[0].config_height, 20);
    }

    /// A bar already at the root's height and margin is not resized again.
    #[test]
    fn bar_matching_the_config_is_not_resized_again() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        state.bars.push(fake.bar("DP-1", 1));
        layouts_sent(&mut state, Some(ModuleId::new(1)));
        let layer_surface = id(&state.bars[0].layer_surface);
        fake.requests();
        assert_eq!(layouts_sent(&mut state, Some(ModuleId::new(1))), 1);
        let resized = fake
            .requests()
            .iter()
            .any(|request| request.object == layer_surface);
        assert!(!resized);
    }
}
