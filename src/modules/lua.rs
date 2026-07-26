use crate::domain::config::ModuleConfig;
use crate::domain::dbus::{BusType, DBusSubscription};
use crate::domain::signals::{SignalHub, SignalKind};
use crate::domain::MonitorId;
use crate::ports::registry::AnyModulePort;
use mlua::{Function, Lua, LuaSerdeExt, UserData, UserDataMethods};
use std::sync::Mutex;

#[derive(Clone)]
pub struct LuaMonitor(pub MonitorId);

impl UserData for LuaMonitor {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| Ok(this.0.as_str().to_string()));
    }
}

pub struct LuaScriptLoader;

impl LuaScriptLoader {
    pub fn load_built_in(name: &str) -> Option<String> {
        match name {
            "hour" => Some(include_str!("builtins/hour.lua").to_string()),
            "workspace" => Some(include_str!("builtins/workspace.lua").to_string()),
            "applet" => Some(include_str!("builtins/applet.lua").to_string()),
            "metrics" => Some(include_str!("builtins/metrics.lua").to_string()),
            _ => None,
        }
    }

    pub fn load_external(name: &str) -> Option<String> {
        let home = std::env::var("HOME").ok()?;
        let path = std::path::PathBuf::from(home)
            .join(".config/cranky/modules")
            .join(format!("{}.lua", name));

        if path.exists() {
            std::fs::read_to_string(path).ok()
        } else {
            None
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
                    if let Ok(val) = lua.to_value(&dbus_state.properties) { globals.set("dbus", val)?; }
                    dbus_handled = true;
                }
                SignalKind::Applets => {
                    let applets = hub.applets_rx().borrow().clone();
                    let items = applets.items().values().collect::<Vec<_>>();
                    match lua.to_value(&items) {
                        Ok(val) => {
                            globals.set("applets", val)?;
                        }
                        Err(e) => println!("Failed to serialize applets: {:?}", e),
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
            let _ = refresh_fn.call::<()>(());
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

    pub fn built_in(name: &str) -> Option<Self> {
        LuaScriptLoader::load_built_in(name).map(|source| Self::new(name.to_string(), source))
    }

    pub fn external(name: &str) -> Option<Self> {
        LuaScriptLoader::load_external(name).map(|source| Self::new(name.to_string(), source))
    }
}

impl AnyModulePort for LuaModule {
    fn init(&mut self, config: &ModuleConfig, full_config: &crate::domain::config::Config) -> Result<(), String> {
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
                                    subs.push(SignalKind::DBus(DBusSubscription {
                                        bus,
                                        destination: dbus_sub.get::<String>("destination").ok(),
                                        path: dbus_sub.get::<String>("path").ok(),
                                        interface: dbus_sub.get::<String>("interface").ok(),
                                        member: dbus_sub.get::<String>("member").ok(),
                                    }));
                                }
                    }
                }
        subs
    }

    fn refresh(&mut self, hub: &SignalHub, changed: &[SignalKind]) {
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let _ = LuaStateSynchronizer::sync(&lua, hub, changed);
    }

    fn render(&self, monitor: &MonitorId) -> crate::domain::layout::LayoutNode {
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let globals = lua.globals();
        let lua_monitor = LuaMonitor(monitor.clone());

        if let Ok(render_fn) = globals.get::<mlua::Function>("render") {
            match render_fn.call::<mlua::Value>(lua_monitor) {
                Ok(val) => {
                    match lua.from_value::<crate::domain::layout::LayoutNode>(val) {
                        Ok(node) => return node,
                        Err(e) => {
                            let msg = format!("Deserialization error: {}", e);
                            return crate::domain::layout::LayoutNode::Flex { 
                                children: vec![
                                    crate::domain::layout::LayoutNode::Text {
                                        text: crate::domain::layout::TextContent::new(msg),
                                        color: crate::domain::shared::color::DrawingColor::Solid(crate::domain::shared::color::Color::new(255, 0, 0, 255)),
                                        font: None,
                                        size: None,
                                        on_click: None,
                                        on_hover: None,
                                        tooltip: None,
                                    }
                                ], 
                                style: crate::domain::layout::FlexStyle::default(),
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
                    println!("Lua render_fn failed: {}", e);
                }
            }
        }
        
        crate::domain::layout::LayoutNode::Flex { children: vec![], style: crate::domain::layout::FlexStyle::default(), background: None, radius: None, on_click: None, on_hover: None, tooltip: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MonitorId;
    use crate::domain::applets::{
        AppletId, AppletItem, AppletStatus, AppletsState, Destination, ObjectPath, Title,
    };
    use crate::domain::config::ModuleConfig;
    
    
    use std::collections::HashMap;

    #[test]
    fn test_applet_missing_icon_regression() {
        let mut module = LuaModule::built_in("applet").expect("Failed to load applet module");
        let module_config = ModuleConfig::new("applet".into(), true, HashMap::new());
        let config = crate::domain::config::Config::default();

        module
            .init(&module_config, &config)
            .expect("Init failed");

        let hub = SignalHub::new(crate::domain::config::Config::default());
        let item = AppletItem::new(crate::domain::applets::CreateAppletCommand {
            id: AppletId::new("test_applet"),
            destination: Destination::new("dest"),
            path: ObjectPath::new("/path"),
            title: Title::new("Test Applet"),
            status: AppletStatus::Active,
            icon_name: None,
            icon_image: None,
            menu_path: None,
        });

        let mut map = std::collections::BTreeMap::new();
        map.insert(item.id().clone(), item);
        hub.applets_tx()
            .send(AppletsState::new(map))
            .unwrap();

        let subs = module.subscriptions();
        module.refresh(&hub, &subs);

        let layout = module.render(&MonitorId::new("DP-1")); println!("{:#?}", layout);
        
        // Assert it returns a row with a single child (the applet)
        if let crate::domain::layout::LayoutNode::Flex { children, .. } = layout {
            assert_eq!(children.len(), 1);
            let applet_node = &children[0];
            
            // The applet node itself should be a row containing a rect (icon) and text (title)
            if let crate::domain::layout::LayoutNode::Flex { children: applet_children, .. } = applet_node {
                assert_eq!(applet_children.len(), 2);
                assert!(matches!(applet_children[0], crate::domain::layout::LayoutNode::Rect { .. }));
                assert!(matches!(applet_children[1], crate::domain::layout::LayoutNode::Text { .. }));
            } else {
                panic!("Applet node is not a Flex");
            }
        } else {
            panic!("Root node is not a Row");
        }
    }
}
