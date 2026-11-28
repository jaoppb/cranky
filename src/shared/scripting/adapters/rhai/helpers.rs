use crate::shared::primitives::ScriptMonitorInfo;
use rhai::{Dynamic, Scope};

pub(super) fn get_hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| {
            std::fs::read_to_string("/etc/hostname").map_or_else(
                |_| "unknown".to_string(),
                |contents| contents.trim().to_string(),
            )
        })
}

pub(super) fn monitor_info_to_rhai_map(info: &ScriptMonitorInfo) -> rhai::Map {
    let mut map = rhai::Map::new();
    let size = info.size();
    map.insert("id".into(), Dynamic::from(info.id().as_str().to_string()));
    map.insert("name".into(), Dynamic::from(info.display_name()));
    map.insert("width".into(), Dynamic::from(i64::from(size.width())));
    map.insert("height".into(), Dynamic::from(i64::from(size.height())));
    map.insert(
        "scale".into(),
        Dynamic::from(f64::from(info.scale().value())),
    );
    map.insert("is_focused".into(), Dynamic::from(info.is_focused()));
    map.insert(
        "active_workspace_id".into(),
        workspace_id_value(info.active_workspace_id()),
    );
    map.insert(
        "special_workspace_id".into(),
        workspace_id_value(info.special_workspace_id()),
    );
    map
}

/// A workspace id as a script sees it: an integer, or unit when there is none.
fn workspace_id_value(workspace_id: Option<i32>) -> Dynamic {
    workspace_id.map_or(Dynamic::UNIT, |workspace_id| {
        Dynamic::from(i64::from(workspace_id))
    })
}

/// `props` as an element of the kind `element_type`.
pub(super) fn with_type(mut props: rhai::Map, element_type: &'static str) -> rhai::Map {
    props.insert("type".into(), Dynamic::from(element_type));
    props
}

pub(super) fn create_initial_scope() -> Scope<'static> {
    let mut scope = Scope::new();

    let ui_map = initial_ui_map();
    let signals_map = rhai::Map::new();
    let config_map = rhai::Map::new();
    let sys_map = initial_sys_map();

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

/// The `ui` map a script starts with: an empty `element` map and the `action` map whose `_ns`
/// marks `ui.action` calls.
fn initial_ui_map() -> rhai::Map {
    let mut ui_map = rhai::Map::new();
    let ui_element_map = rhai::Map::new();
    let mut ui_action_map = rhai::Map::new();
    ui_action_map.insert("_ns".into(), Dynamic::from("ui.action"));
    ui_map.insert("element".into(), Dynamic::from(ui_element_map));
    ui_map.insert("action".into(), Dynamic::from(ui_action_map));
    ui_map
}

/// The `sys` map a script starts with: its `_ns` and empty `log` and `monitors` maps.
fn initial_sys_map() -> rhai::Map {
    let mut sys_map = rhai::Map::new();
    sys_map.insert("_ns".into(), Dynamic::from("sys"));
    let sys_log_map = rhai::Map::new();
    let sys_monitors_map = rhai::Map::new();
    sys_map.insert("log".into(), Dynamic::from(sys_log_map));
    sys_map.insert("monitors".into(), Dynamic::from(sys_monitors_map));
    sys_map
}
