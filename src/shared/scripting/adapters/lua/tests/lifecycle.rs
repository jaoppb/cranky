use crate::features::module_runtime::ports::AnyModulePort;
use crate::features::systray::domain::{
    Destination, ObjectPath, SystrayId, SystrayItem, SystrayState, SystrayStatus, Title,
};
use crate::shared::config::domain::ModuleConfig;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::MonitorId;
use crate::shared::scripting::adapters::lua::LuaModule;

#[test]
fn test_systray_missing_icon_regression() {
    let mut module = LuaModule::built_in("systray").expect("Failed to load systray module");
    let module_config = ModuleConfig::new(
        "systray".into(),
        true,
        crate::shared::config::domain::EngineSelection::Auto,
        crate::shared::primitives::ModuleOptions::default(),
    );
    let config = crate::shared::config::domain::Config::default();

    module.init(&module_config, &config).expect("Init failed");

    let hub = SignalHub::new(crate::shared::config::domain::Config::default());
    let item = SystrayItem::new(
        crate::features::systray::domain::CreateSystrayItemCommand::new(
            SystrayId::new("test_systray"),
            Destination::new("dest"),
            ObjectPath::new("/path"),
            Title::new("Test Systray"),
            SystrayStatus::Active,
            None,
            None,
            crate::features::systray::domain::SystrayCategory::ApplicationStatus,
            crate::features::systray::domain::ItemIsMenu::new(false),
        ),
    );

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
    assert_eq!(item_node.children().len(), 2);
    assert_eq!(
        item_node.children()[0].tag(),
        crate::features::vdom::domain::NodeTag::Rect
    );
    assert_eq!(
        item_node.children()[1].tag(),
        crate::features::vdom::domain::NodeTag::Text
    );
}

#[test]
fn test_systray_with_icon_renders_image() {
    let mut module = LuaModule::built_in("systray").expect("Failed to load systray module");
    let module_config = ModuleConfig::new(
        "systray".into(),
        true,
        crate::shared::config::domain::EngineSelection::Auto,
        crate::shared::primitives::ModuleOptions::default(),
    );
    let config = crate::shared::config::domain::Config::default();
    module.init(&module_config, &config).expect("Init failed");

    let hub = SignalHub::new(crate::shared::config::domain::Config::default());
    let icon_img = crate::features::systray::domain::IconImage::new(
        vec![255; 16 * 16 * 4],
        crate::shared::primitives::geometry::Size::new(16, 16),
    );
    let icon = crate::features::systray::domain::SystrayIcon::new(
        Some(crate::features::systray::domain::IconName::new("test-icon")),
        Some(icon_img),
    );

    let item = SystrayItem::new(
        crate::features::systray::domain::CreateSystrayItemCommand::new(
            SystrayId::new("test_systray"),
            Destination::new("dest"),
            ObjectPath::new("/path"),
            Title::new("Test Systray"),
            SystrayStatus::Active,
            icon,
            None,
            crate::features::systray::domain::SystrayCategory::ApplicationStatus,
            crate::features::systray::domain::ItemIsMenu::new(false),
        ),
    );

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
    assert_eq!(item_node.children().len(), 2);
    assert_eq!(
        item_node.children()[0].tag(),
        crate::features::vdom::domain::NodeTag::Image
    );
    assert_eq!(
        item_node.children()[1].tag(),
        crate::features::vdom::domain::NodeTag::Text
    );
}

#[test]
fn test_calendar_lua_config_monday() {
    let mut module = LuaModule::built_in("calendar").expect("Failed to load calendar module");
    let mut opts = std::collections::HashMap::new();
    opts.insert(
        "first_day_of_week".to_string(),
        crate::shared::primitives::DynamicValue::String("monday".to_string()),
    );
    let module_config = ModuleConfig::new(
        "calendar".into(),
        true,
        crate::shared::config::domain::EngineSelection::Auto,
        crate::shared::primitives::ModuleOptions::new(opts),
    );
    let config = crate::shared::config::domain::Config::default();
    module.init(&module_config, &config).expect("Init failed");

    let hub = SignalHub::new(crate::shared::config::domain::Config::default());
    let test_time = chrono::DateTime::parse_from_rfc3339("2026-09-01T12:00:00+00:00")
        .unwrap()
        .with_timezone(&chrono::Local);
    hub.time_tx().send(test_time).unwrap();
    let subs = module.subscriptions().to_vec();
    module.refresh(&hub, &subs);

    let node = module.render(&MonitorId::new("DP-1"));
    let weekdays = &node.children()[1];
    if let crate::features::vdom::domain::VNodeKind::Text { text } =
        weekdays.children()[0].kind()
    {
        assert_eq!(
            text.as_str(),
            "Mo",
            "First weekday should be Mo for monday config"
        );
    } else {
        panic!("Expected text node for weekday");
    }
}
