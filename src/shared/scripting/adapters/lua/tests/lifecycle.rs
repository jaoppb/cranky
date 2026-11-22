use crate::features::module_runtime::ports::AnyModulePort;
use crate::features::systray::domain::{
    Destination, ObjectPath, SystrayId, SystrayItem, SystrayState, SystrayStatus, Title,
};
use crate::shared::config::domain::ModuleConfig;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::MonitorId;
use crate::shared::scripting::adapters::lua::LuaModule;

/// `module` initialised with `options` and the default configuration.
fn initialised(
    mut module: LuaModule,
    options: crate::shared::primitives::ModuleOptions,
) -> LuaModule {
    let module_config = ModuleConfig::new(module.name.as_str().into(), options);
    let config = crate::shared::config::domain::Config::default();
    module.init(&module_config, &config).expect("Init failed");
    module
}

/// The built-in Lua module `name`, initialised with `options` and the default configuration.
fn init_built_in(name: &str, options: crate::shared::primitives::ModuleOptions) -> LuaModule {
    initialised(
        LuaModule::built_in(name).expect("Failed to load built-in module"),
        options,
    )
}

/// An active tray item, with `icon` when given.
fn systray_item(icon: Option<crate::features::systray::domain::SystrayIcon>) -> SystrayItem {
    let params = crate::features::systray::domain::SystrayItemParams::new(
        SystrayId::new("test_systray"),
        crate::features::systray::domain::SystrayAddress::new(
            Destination::new("dest"),
            ObjectPath::new("/path"),
        ),
        Title::new("Test Systray"),
    )
    .with_status(SystrayStatus::Active);
    let params = match icon {
        Some(_) => params.with_icon(icon),
        None => params,
    };
    SystrayItem::new(
        crate::features::systray::domain::CreateSystrayItemCommand::new(
            params
                .with_category(crate::features::systray::domain::SystrayCategory::ApplicationStatus)
                .with_item_is_menu(crate::features::systray::domain::ItemIsMenu::Activatable),
        ),
    )
}

/// The tags of the children of the one item the built-in systray module renders for `item`,
/// after checking the layout holds one flex item.
fn rendered_systray_item_tags(item: SystrayItem) -> Vec<crate::features::vdom::domain::NodeTag> {
    let mut module = init_built_in(
        "systray",
        crate::shared::primitives::ModuleOptions::default(),
    );
    let hub = SignalHub::new(crate::shared::config::domain::Config::default());
    let mut map = std::collections::BTreeMap::new();
    map.insert(item.id().clone(), item);
    hub.systray_tx().send(SystrayState::new(map)).unwrap();
    let subs = module.subscriptions().to_vec();
    module.refresh(&hub, &subs);

    let layout = module.render(&MonitorId::new("DP-1"));
    assert_eq!(layout.tag(), crate::features::vdom::domain::NodeTag::Flex);
    assert_eq!(layout.children().len(), 1);
    let item_node = &layout.children()[0];
    assert_eq!(
        item_node.tag(),
        crate::features::vdom::domain::NodeTag::Flex
    );
    item_node
        .children()
        .iter()
        .map(crate::features::vdom::domain::VNode::tag)
        .collect()
}

#[test]
fn test_systray_missing_icon_regression() {
    use crate::features::vdom::domain::NodeTag;
    assert_eq!(
        rendered_systray_item_tags(systray_item(None)),
        [NodeTag::Rect, NodeTag::Text]
    );
}

#[test]
fn test_systray_with_icon_renders_image() {
    use crate::features::vdom::domain::NodeTag;
    let icon_img = crate::features::systray::domain::IconImage::new(
        vec![255; 16 * 16 * 4],
        crate::shared::primitives::geometry::Size::new(16, 16),
    );
    let icon = crate::features::systray::domain::SystrayIcon::new(
        Some(crate::features::systray::domain::IconName::new("test-icon")),
        Some(icon_img),
    );
    assert_eq!(
        rendered_systray_item_tags(systray_item(icon)),
        [NodeTag::Image, NodeTag::Text]
    );
}

#[test]
fn test_calendar_lua_config_monday() {
    let mut opts = std::collections::HashMap::new();
    opts.insert(
        "first_day_of_week".to_string(),
        crate::shared::primitives::DynamicValue::String("monday".to_string()),
    );
    let mut module = init_built_in(
        "calendar",
        crate::shared::primitives::ModuleOptions::new(opts),
    );

    let hub = SignalHub::new(crate::shared::config::domain::Config::default());
    // 2026-09-01T12:00:00Z
    let test_time = cranky_time::LocalTime::utc(1_788_264_000);
    hub.time_tx().send(test_time).unwrap();
    let subs = module.subscriptions().to_vec();
    module.refresh(&hub, &subs);

    let node = module.render(&MonitorId::new("DP-1"));
    let weekdays = &node.children()[1];
    if let crate::features::vdom::domain::VNodeKind::Text { text } = weekdays.children()[0].kind() {
        assert_eq!(
            text.as_str(),
            "Mo",
            "First weekday should be Mo for monday config"
        );
    } else {
        panic!("Expected text node for weekday");
    }
}

const TIME_TEXT_SOURCE: &str = r#"
        function init() end
        function metadata() return { subscriptions = { "time" }, styles = {} } end
        function refresh() end
        function render(monitor) return ui.text({ text = signals.time }) end
    "#;

#[test]
fn test_time_signal_is_whole_second_rfc_3339_with_a_numeric_offset() {
    let mut module = initialised(
        LuaModule::new("time_text".into(), TIME_TEXT_SOURCE.to_string()),
        crate::shared::primitives::ModuleOptions::default(),
    );

    let hub = SignalHub::new(crate::shared::config::domain::Config::default());
    // 2026-11-18T11:06:40Z, three hours west of UTC.
    let instant = std::time::UNIX_EPOCH
        .checked_add(std::time::Duration::from_secs(1_795_000_000))
        .unwrap();
    let west = cranky_time::Environment::new(Some("<-03>3"), "/nonexistent", Vec::new());
    hub.time_tx()
        .send(cranky_time::Clock::new().at_instant(instant, &west))
        .unwrap();
    module.refresh(&hub, &[crate::shared::events::signals::SignalKind::Time]);

    let node = module.render(&MonitorId::new("DP-1"));
    let crate::features::vdom::domain::VNodeKind::Text { text } = node.kind() else {
        panic!("Expected a text node, got {:?}", node.tag());
    };
    assert_eq!(text.as_str(), "2026-11-18T08:06:40-03:00");
}
