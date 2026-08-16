use crate::shared::config::domain::ModuleConfig;
use crate::shared::dbus::domain::{BusType, DBusSubscription};
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::MonitorId;
use crate::features::module_runtime::ports::AnyModulePort;
use mlua::{Function, Lua, LuaSerdeExt, UserData, UserDataMethods};
use std::sync::Mutex;

#[derive(Clone)]
pub struct LuaMonitor(pub MonitorId);

impl UserData for LuaMonitor {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| Ok(this.0.as_str().to_string()));
    }
}

#[cfg(test)]
pub struct LuaScriptLoader;

#[cfg(test)]
impl LuaScriptLoader {
    pub fn load_built_in(name: &str) -> Option<String> {
        match name {
            "hour" => Some(include_str!("../../../../assets/widgets/hour.lua").to_string()),
            "workspace" => Some(include_str!("../../../../assets/widgets/workspace.lua").to_string()),
            "applet" => Some(include_str!("../../../../assets/widgets/applet.lua").to_string()),
            "metrics" => Some(include_str!("../../../../assets/widgets/metrics.lua").to_string()),
            _ => None,
        }
    }

}

pub struct LuaStateSynchronizer;

impl LuaStateSynchronizer {
    pub fn sync(lua: &Lua, hub: &SignalHub, changed: &[SignalKind]) -> mlua::Result<()> {
        let globals = lua.globals();
        let mut dbus_handled = false;

        for signal in changed {
            match signal {
                SignalKind::Time => {
                    let time = *hub.time_rx().borrow();
                    globals.set("current_time", time.to_rfc3339())?;
                }
                SignalKind::Hyprland => {
                    let hypr = hub.hyprland_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&hypr) { globals.set("hyprland", val)?; }
                }
                SignalKind::DBus(_) if !dbus_handled => {
                    let dbus_state = hub.dbus_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&dbus_state.properties()) { globals.set("dbus", val)?; }
                    dbus_handled = true;
                }
                SignalKind::Applets => {
                    let t0 = std::time::Instant::now();
                    let applets = hub.applets_rx().borrow().clone();
                    let items = applets.items().values().collect::<Vec<_>>();
                    let item_count = items.len();
                    tracing::debug!(item_count, "Serializing applets to Lua");
                    match lua.to_value(&items) {
                        Ok(val) => {
                            globals.set("applets", val)?;
                            tracing::debug!(
                                item_count,
                                duration_ms = t0.elapsed().as_millis(),
                                duration_micros = t0.elapsed().as_micros(),
                                "Successfully serialized applets to Lua"
                            );
                        }
                        Err(e) => tracing::error!(err = ?e, "Failed to serialize applets to Lua"),
                    }
                }
                SignalKind::Metrics => {
                    let metrics = hub.metrics_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&metrics) { globals.set("metrics", val)?; }
                }
                _ => {}
            }
        }

        if let Ok(refresh_fn) = globals.get::<Function>("refresh") {
            let t0 = std::time::Instant::now();
            match refresh_fn.call::<()>(()) {
                Ok(_) => {
                    tracing::debug!(
                        duration_ms = t0.elapsed().as_millis(),
                        duration_micros = t0.elapsed().as_micros(),
                        "Lua refresh function called successfully"
                    );
                }
                Err(e) => {
                    tracing::error!(err = ?e, "Lua refresh function failed");
                }
            }
        }

        Ok(())
    }
}

pub struct LuaModule {
    lua: Mutex<Lua>,
    source: String,
    name: String,
}

impl LuaModule {
    pub fn new(name: String, source: String) -> Self {
        Self {
            lua: Mutex::new(Lua::new()),
            source,
            name,
        }
    }

    #[cfg(test)]
    pub fn built_in(name: &str) -> Option<Self> {
        LuaScriptLoader::load_built_in(name).map(|source| Self::new(name.to_string(), source))
    }
}

impl AnyModulePort for LuaModule {
    fn init(&mut self, config: &ModuleConfig, full_config: &crate::shared::config::domain::Config) -> Result<(), String> {
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let globals = lua.globals();

        let bar_config = full_config.bar();

        // Expose bar config
        let bar_config_table = lua.create_table().map_err(|e| e.to_string())?;
        bar_config_table
            .set("font_family", bar_config.font_family().as_str())
            .map_err(|e| e.to_string())?;
        bar_config_table
            .set("font_size", bar_config.font_size().value())
            .map_err(|e| e.to_string())?;
        globals
            .set("bar_config", bar_config_table)
            .map_err(|e| e.to_string())?;

        // Expose module config options using mlua's serde support
        let options_lua = lua
            .to_value(config.options())
            .map_err(|e| format!("Failed to convert config to Lua: {}", e))?;
        globals
            .set("config", options_lua)
            .map_err(|e| e.to_string())?;

        // Load the script
        lua.load(&self.source)
            .set_name(&self.name)
            .exec()
            .map_err(|e| format!("Lua load error in {}: {}", self.name, e))?;

        // Call init if it exists
        if let Ok(init_fn) = globals.get::<Function>("init") {
            init_fn
                .call::<()>(())
                .map_err(|e| format!("Lua init error in {}: {}", self.name, e))?;
        }

        Ok(())
    }

    fn subscriptions(&self) -> Vec<SignalKind> {
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let globals = lua.globals();

        let mut subs = Vec::new();
        if let Ok(subs_fn) = globals.get::<Function>("subscriptions")
            && let Ok(result) = subs_fn.call::<mlua::Value>(())
                && let mlua::Value::Table(t) = result {
                    for (_, val) in t.pairs::<mlua::Value, mlua::Value>().flatten() {
                        if let mlua::Value::String(s) = &val {
                            if let Ok(s_str) = s.to_str() {
                                match s_str.as_ref() {
                                    "time" => subs.push(SignalKind::Time),
                                    "hyprland" => subs.push(SignalKind::Hyprland),
                                    "applets" => subs.push(SignalKind::Applets),
                                    "metrics" => subs.push(SignalKind::Metrics),
                                    _ => {}
                                }
                            }
                        } else if let mlua::Value::Table(dbus_sub) = &val
                            && let Ok(typ) = dbus_sub.get::<String>("type")
                                && typ == "dbus" {
                                    let bus_str = dbus_sub
                                        .get::<String>("bus")
                                        .unwrap_or_else(|_| "session".to_string());
                                    let bus = if bus_str == "system" {
                                        BusType::System
                                    } else {
                                        BusType::Session
                                    };
                                    subs.push(SignalKind::DBus(DBusSubscription::new(
                                        bus,
                                        dbus_sub.get::<String>("destination").ok().map(crate::shared::dbus::domain::Destination::new),
                                        dbus_sub.get::<String>("path").ok().map(crate::shared::dbus::domain::Path::new),
                                        dbus_sub.get::<String>("interface").ok().map(crate::shared::dbus::domain::Interface::new),
                                        dbus_sub.get::<String>("member").ok().map(crate::shared::dbus::domain::Member::new),
                                    )));
                                }
                    }
                }
        subs
    }

    fn refresh(&mut self, hub: &SignalHub, changed: &[SignalKind]) {
        let t0 = std::time::Instant::now();
        tracing::debug!(?changed, "Refreshing LuaModulePort");
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        match LuaStateSynchronizer::sync(&lua, hub, changed) {
            Ok(_) => {
                tracing::debug!(
                    ?changed,
                    duration_ms = t0.elapsed().as_millis(),
                    duration_micros = t0.elapsed().as_micros(),
                    "LuaModulePort refresh completed successfully"
                );
            }
            Err(e) => {
                tracing::error!(?changed, err = ?e, "LuaModulePort refresh failed");
            }
        }
    }

    fn render(&self, monitor: &MonitorId) -> crate::features::layout_engine::domain::LayoutNode {
        let t0 = std::time::Instant::now();
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let globals = lua.globals();
        let lua_monitor = LuaMonitor(monitor.clone());

        if let Ok(render_fn) = globals.get::<mlua::Function>("render") {
            match render_fn.call::<mlua::Value>(lua_monitor) {
                Ok(val) => {
                    let call_ms = t0.elapsed().as_millis();
                    match lua.from_value::<crate::features::layout_engine::domain::LayoutNode>(val) {
                        Ok(node) => {
                            tracing::debug!(
                                monitor = %monitor,
                                call_ms,
                                total_ms = t0.elapsed().as_millis(),
                                ?node,
                                "Lua render_fn succeeded and deserialized layout node"
                            );
                            return node;
                        }
                        Err(e) => {
                            tracing::error!(monitor = %monitor, err = ?e, "Failed to deserialize LayoutNode from Lua render_fn");
                            let msg = format!("Deserialization error: {}", e);
                            return crate::features::layout_engine::domain::LayoutNode::Flex { 
                                children: vec![
                                    crate::features::layout_engine::domain::LayoutNode::Text {
                                        text: crate::features::layout_engine::domain::TextContent::new(msg),
                                        color: crate::shared::primitives::color::DrawingColor::Solid(crate::shared::primitives::color::Color::new(255, 0, 0, 255)),
                                        font: None,
                                        size: None,
                                        on_click: None,
                                        on_hover: None,
                                        tooltip: None,
                                    }
                                ], 
                                style: crate::features::layout_engine::domain::FlexStyle::default(),
                                background: None,
                                radius: None,
                                on_click: None, 
                                on_hover: None,
                                tooltip: None,
                            };
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(monitor = %monitor, err = ?e, "Lua render_fn execution failed");
                }
            }
        }
        
        crate::features::layout_engine::domain::LayoutNode::Flex { children: vec![], style: crate::features::layout_engine::domain::FlexStyle::default(), background: None, radius: None, on_click: None, on_hover: None, tooltip: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::primitives::MonitorId;
    use crate::features::applets::domain::{
        AppletId, AppletItem, AppletStatus, AppletsState, Destination, ObjectPath, Title,
    };
    use crate::shared::config::domain::ModuleConfig;
    
    
    use std::collections::HashMap;

    #[test]
    fn test_applet_missing_icon_regression() {
        let mut module = LuaModule::built_in("applet").expect("Failed to load applet module");
        let module_config = ModuleConfig::new("applet".into(), true, crate::shared::config::domain::EngineSelection::Auto, HashMap::new());
        let config = crate::shared::config::domain::Config::default();

        module
            .init(&module_config, &config)
            .expect("Init failed");

        let hub = SignalHub::new(crate::shared::config::domain::Config::default());
        let item = AppletItem::new(crate::features::applets::domain::CreateAppletCommand::new(
            AppletId::new("test_applet"),
            Destination::new("dest"),
            ObjectPath::new("/path"),
            Title::new("Test Applet"),
            AppletStatus::Active,
            None,
            None,
            crate::features::applets::domain::AppletCategory::ApplicationStatus,
            crate::features::applets::domain::ItemIsMenu::new(false),
        ));

        let mut map = std::collections::BTreeMap::new();
        map.insert(item.id().clone(), item);
        hub.applets_tx()
            .send(AppletsState::new(map))
            .unwrap();

        let subs = module.subscriptions();
        module.refresh(&hub, &subs);

        let layout = module.render(&MonitorId::new("DP-1")); println!("{:#?}", layout);
        
        // Assert it returns a row with a single child (the applet)
        if let crate::features::layout_engine::domain::LayoutNode::Flex { children, .. } = layout {
            assert_eq!(children.len(), 1);
            let applet_node = &children[0];
            
            // The applet node itself should be a row containing a rect (icon) and text (title)
            if let crate::features::layout_engine::domain::LayoutNode::Flex { children: applet_children, .. } = applet_node {
                assert_eq!(applet_children.len(), 2);
                assert!(matches!(applet_children[0], crate::features::layout_engine::domain::LayoutNode::Rect { .. }));
                assert!(matches!(applet_children[1], crate::features::layout_engine::domain::LayoutNode::Text { .. }));
            } else {
                panic!("Applet node is not a Flex");
            }
        } else {
            panic!("Root node is not a Row");
        }
    }

    #[test]
    fn test_applet_with_icon_renders_image() {
        let mut module =
            LuaModule::built_in("applet").expect("Failed to load applet module");
        let module_config = ModuleConfig::new("applet".into(), true, crate::shared::config::domain::EngineSelection::Auto, HashMap::new());
        let config = crate::shared::config::domain::Config::default();
        module
            .init(&module_config, &config)
            .expect("Init failed");

        let hub = SignalHub::new(crate::shared::config::domain::Config::default());
        let icon_img = crate::features::applets::domain::IconImage::new(
            vec![255; 16 * 16 * 4],
            crate::shared::primitives::geometry::Size::new(16, 16),
        );
        let icon = crate::features::applets::domain::AppletIcon::new(
            Some(crate::features::applets::domain::IconName::new("test-icon")),
            Some(icon_img),
        );

        let item = AppletItem::new(crate::features::applets::domain::CreateAppletCommand::new(
            AppletId::new("test_applet"),
            Destination::new("dest"),
            ObjectPath::new("/path"),
            Title::new("Test Applet"),
            AppletStatus::Active,
            icon,
            None,
            crate::features::applets::domain::AppletCategory::ApplicationStatus,
            crate::features::applets::domain::ItemIsMenu::new(false),
        ));

        let mut map = std::collections::BTreeMap::new();
        map.insert(item.id().clone(), item);
        hub.applets_tx()
            .send(AppletsState::new(map))
            .unwrap();

        let subs = module.subscriptions();
        module.refresh(&hub, &subs);

        let layout = module.render(&MonitorId::new("DP-1"));
        if let crate::features::layout_engine::domain::LayoutNode::Flex { children, .. } = layout {
            assert_eq!(children.len(), 1);
            let applet_node = &children[0];
            if let crate::features::layout_engine::domain::LayoutNode::Flex {
                children: applet_children,
                ..
            } = applet_node
            {
                assert_eq!(applet_children.len(), 2);
                assert!(matches!(
                    applet_children[0],
                    crate::features::layout_engine::domain::LayoutNode::Image { .. }
                ));
                assert!(matches!(
                    applet_children[1],
                    crate::features::layout_engine::domain::LayoutNode::Text { .. }
                ));
            } else {
                panic!("Applet node is not a Flex");
            }
        } else {
            panic!("Root node is not a Row");
        }
    }
}
