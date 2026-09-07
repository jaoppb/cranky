use crate::shared::primitives::ScriptMonitorInfo;
use rhai::{Dynamic, Scope};

pub(crate) fn get_hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| {
            std::fs::read_to_string("/etc/hostname")
                .map_or_else(|_| "unknown".to_string(), |s| s.trim().to_string())
        })
}

pub(crate) fn monitor_info_to_rhai_map(info: &ScriptMonitorInfo) -> rhai::Map {
    let mut map = rhai::Map::new();
    let name_str = if info.name().is_empty() {
        info.id().as_str().to_string()
    } else {
        info.name().to_string()
    };
    map.insert("id".into(), Dynamic::from(info.id().as_str().to_string()));
    map.insert("name".into(), Dynamic::from(name_str));
    map.insert(
        "width".into(),
        Dynamic::from(i64::from(info.size().width())),
    );
    map.insert(
        "height".into(),
        Dynamic::from(i64::from(info.size().height())),
    );
    map.insert(
        "scale".into(),
        Dynamic::from(f64::from(info.scale().value())),
    );
    map.insert("is_focused".into(), Dynamic::from(info.is_focused()));
    map.insert(
        "active_workspace_id".into(),
        info.active_workspace_id()
            .map_or(Dynamic::UNIT, |w| Dynamic::from(i64::from(w))),
    );
    map.insert(
        "special_workspace_id".into(),
        info.special_workspace_id()
            .map_or(Dynamic::UNIT, |w| Dynamic::from(i64::from(w))),
    );
    map
}

pub(crate) fn create_initial_scope() -> Scope<'static> {
    let mut scope = Scope::new();

    // ui
    let mut ui_map = rhai::Map::new();
    let ui_element_map = rhai::Map::new();
    let mut ui_action_map = rhai::Map::new();
    ui_action_map.insert("_ns".into(), Dynamic::from("ui.action"));
    ui_map.insert("element".into(), Dynamic::from(ui_element_map));
    ui_map.insert("action".into(), Dynamic::from(ui_action_map));

    // signals
    let signals_map = rhai::Map::new();

    // config
    let config_map = rhai::Map::new();

    // sys
    let mut sys_map = rhai::Map::new();
    sys_map.insert("_ns".into(), Dynamic::from("sys"));
    let sys_log_map = rhai::Map::new();
    let sys_monitors_map = rhai::Map::new();
    sys_map.insert("log".into(), Dynamic::from(sys_log_map));
    sys_map.insert("monitors".into(), Dynamic::from(sys_monitors_map));

    // cranky
    let mut cranky_map = rhai::Map::new();
    cranky_map.insert("ui".into(), Dynamic::from(ui_map.clone()));
    cranky_map.insert("signals".into(), Dynamic::from(signals_map.clone()));
    cranky_map.insert("config".into(), Dynamic::from(config_map.clone()));
    cranky_map.insert("sys".into(), Dynamic::from(sys_map.clone()));

    scope.push("cranky", cranky_map);
    scope.push("ui", ui_map);
    scope.push("signals", signals_map);
    scope.push("config", config_map);
    scope.push("sys", sys_map);

    scope
}
