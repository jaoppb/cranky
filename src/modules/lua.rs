use mlua::{Lua, UserData, UserDataMethods, LuaSerdeExt, Function, Value};
use crate::ports::canvas::Canvas;
use crate::domain::signals::{SignalHub, SignalKind};
use crate::domain::dbus::{BusType, DBusSubscription};
use crate::domain::config::{ModuleConfig, BarConfig};
use crate::domain::{ModuleId, MonitorId, geometry::Size};
use crate::modules::ModuleError;
use crate::ports::registry::AnyModulePort;
use crate::domain::color::DrawingColor;
use std::sync::Mutex;
use std::cell::RefCell;

#[derive(Clone)]
pub struct LuaMonitor(pub MonitorId);

impl UserData for LuaMonitor {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| {
            Ok(this.0.as_str().to_string())
        });
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
        let source = match name {
            "hour" => include_str!("builtins/hour.lua"),
            "workspace" => include_str!("builtins/workspace.lua"),
            "applet" => include_str!("builtins/applet.lua"),
            "metrics" => include_str!("builtins/metrics.lua"),
            _ => return None,
        };
        Some(Self::new(name.to_string(), source.to_string()))
    }

    pub fn external(name: &str) -> Option<Self> {
        let home = std::env::var("HOME").ok()?;
        let path = std::path::PathBuf::from(home)
            .join(".config/cranky/modules")
            .join(format!("{}.lua", name));
        
        if path.exists() {
            let source = std::fs::read_to_string(path).ok()?;
            Some(Self::new(name.to_string(), source))
        } else {
            None
        }
    }
}

impl AnyModulePort for LuaModule {
    fn init(
        &mut self,
        config: &ModuleConfig,
        bar_config: &BarConfig,
    ) -> Result<(), String> {
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let globals = lua.globals();
        
        // Expose bar config
        let bar_config_table = lua.create_table().map_err(|e| e.to_string())?;
        bar_config_table.set("font_family", bar_config.font_family().as_str()).map_err(|e| e.to_string())?;
        bar_config_table.set("font_size", bar_config.font_size().value()).map_err(|e| e.to_string())?;
        globals.set("bar_config", bar_config_table).map_err(|e| e.to_string())?;

        // Expose module config options using mlua's serde support
        let options_lua = lua.to_value(config.options())
            .map_err(|e| format!("Failed to convert config to Lua: {}", e))?;
        globals.set("config", options_lua).map_err(|e| e.to_string())?;

        // Load the script
        lua.load(&self.source)
            .set_name(&self.name)
            .exec()
            .map_err(|e| format!("Lua load error in {}: {}", self.name, e))?;

        // Call init if it exists
        if let Ok(init_fn) = globals.get::<Function>("init") {
            init_fn.call::<()>(()).map_err(|e| format!("Lua init error in {}: {}", self.name, e))?;
        }

        Ok(())
    }

    fn subscriptions(&self) -> Vec<SignalKind> {
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let globals = lua.globals();
        
        let mut subs = Vec::new();
        if let Ok(subs_fn) = globals.get::<Function>("subscriptions") {
            if let Ok(result) = subs_fn.call::<mlua::Value>(()) {
                if let mlua::Value::Table(t) = result {
                    for pair in t.pairs::<mlua::Value, mlua::Value>() {
                        if let Ok((_, val)) = pair {
                            if let mlua::Value::String(s) = &val {
                                if let Ok(s_str) = s.to_str() {
                                    match s_str.as_ref() {
                                        "time" => subs.push(SignalKind::Time),
                                        "hyprland" => subs.push(SignalKind::Hyprland),
                                        "applets" => subs.push(SignalKind::Applets),
                                        _ => {}
                                    }
                                }
                            } else if let mlua::Value::Table(dbus_sub) = &val {
                                if let Ok(typ) = dbus_sub.get::<String>("type") {
                                    if typ == "dbus" {
                                        let bus_str = dbus_sub.get::<String>("bus").unwrap_or_else(|_| "session".to_string());
                                        let bus = if bus_str == "system" { BusType::System } else { BusType::Session };
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
                        }
                    }
                }
            }
        } else {
            if self.name == "hour" {
                subs.push(SignalKind::Time);
            } else if self.name == "workspace" {
                subs.push(SignalKind::Hyprland);
            }
        }
        subs
    }

    fn refresh(&mut self, hub: &SignalHub) {
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let globals = lua.globals();
        
        let time = *hub.time_rx().borrow();
        let _ = globals.set("current_time", time.to_rfc3339());

        let hypr = hub.hyprland_rx().borrow().clone();
        if let Ok(hypr_lua) = lua.to_value(&hypr) {
            let _ = globals.set("hyprland", hypr_lua);
        }
        
        let dbus_state = hub.dbus_rx().borrow().clone();
        if let Ok(dbus_lua) = lua.to_value(&dbus_state.properties) {
            let _ = globals.set("dbus", dbus_lua);
        }
        
        let applets_state = hub.applets_rx().borrow().clone();
        if let Ok(applets_lua) = lua.to_value(&applets_state.items) {
            let _ = globals.set("applets", applets_lua);
        }
        
        if let Ok(refresh_fn) = globals.get::<Function>("refresh") {
            let _ = refresh_fn.call::<()>(());
        }
    }

    fn view(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) {
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let canvas_cell = RefCell::new(canvas);
        let mut with_canvas = |f: &mut dyn FnMut(&mut dyn Canvas)| f(*canvas_cell.borrow_mut());
        
        let _ = lua.scope(|scope| {
            let globals = lua.globals();
            let lua_canvas = match lua.create_table() {
                Ok(t) => t,
                Err(e) => { tracing::error!("Failed to create lua canvas table: {}", e); return Ok(()); }
            };
            
            let draw_rect = scope.create_function(|_, (_self, x, y, w, h, color_str, radius): (mlua::Value, f32, f32, f32, f32, String, f32)| {
                if let Ok(color) = DrawingColor::parse(&color_str) {
                    with_canvas(&mut |c| c.draw_rect(x, y, w, h, color.clone(), radius));
                }
                Ok(())
            }).unwrap_or_else(|_| scope.create_function(|_, ()| Ok(())).unwrap());
            let _ = lua_canvas.set("draw_rect", draw_rect);

            let draw_border = scope.create_function(|_, (_self, x, y, w, h, color_str, radius, size): (mlua::Value, f32, f32, f32, f32, String, f32, f32)| {
                if let Ok(color) = DrawingColor::parse(&color_str) {
                    with_canvas(&mut |c| c.draw_border(x, y, w, h, color.clone(), radius, size));
                }
                Ok(())
            }).unwrap_or_else(|_| scope.create_function(|_, ()| Ok(())).unwrap());
            let _ = lua_canvas.set("draw_border", draw_border);

            let draw_text = scope.create_function(|_, (_self, text, font, size, color_str, x, y): (mlua::Value, String, String, f32, String, f32, f32)| {
                if let Ok(color) = DrawingColor::parse(&color_str) {
                    with_canvas(&mut |c| c.draw_text(&text, &font, size, color.clone(), x, y));
                }
                Ok(())
            }).unwrap_or_else(|_| scope.create_function(|_, ()| Ok(())).unwrap());
            let _ = lua_canvas.set("draw_text", draw_text);

            let draw_image = scope.create_function(|lua, (_self, image_data_val, width, height, logical_width, logical_height, x, y): (mlua::Value, mlua::Value, u32, u32, f32, f32, f32, f32)| {
                let image_data: Vec<crate::domain::color::Color> = lua.from_value(image_data_val).unwrap_or_default();
                with_canvas(&mut |c| c.draw_image(&image_data, width, height, logical_width, logical_height, x, y));
                Ok(())
            }).unwrap_or_else(|_| scope.create_function(|_, ()| Ok(())).unwrap());
            let _ = lua_canvas.set("draw_image", draw_image);

            let measure_text = scope.create_function(|_, (_self, text, font, size): (mlua::Value, String, String, f32)| {
                let mut res = (0.0, 0.0);
                with_canvas(&mut |c| res = c.measure_text(&text, &font, size));
                Ok(res)
            }).unwrap_or_else(|_| scope.create_function(|_, ()| Ok((0.0, 0.0))).unwrap());
            let _ = lua_canvas.set("measure_text", measure_text);

            let lua_monitor = LuaMonitor(monitor.clone());
            
            if let Ok(view_fn) = globals.get::<Function>("view") {
                let _ = view_fn.call::<()>((lua_canvas, lua_monitor)).unwrap_or_else(|e| {
                    eprintln!("Lua view error in {}: {}", self.name, e);
                });
            }
            Ok(())
        });
    }

    fn measure(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) -> Size {
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let canvas_cell = RefCell::new(canvas);
        let mut with_canvas = |f: &mut dyn FnMut(&mut dyn Canvas)| f(*canvas_cell.borrow_mut());
        
        let res = lua.scope(|scope| {
            let globals = lua.globals();
            let lua_canvas = match lua.create_table() {
                Ok(t) => t,
                Err(e) => { tracing::error!("Failed to create lua canvas table: {}", e); return Ok(Size::new(0, 0)); }
            };
            
            let measure_text = scope.create_function(|_, (_self, text, font, size): (mlua::Value, String, String, f32)| {
                let mut res = (0.0, 0.0);
                with_canvas(&mut |c| res = c.measure_text(&text, &font, size));
                Ok(res)
            }).unwrap_or_else(|_| scope.create_function(|_, ()| Ok((0.0, 0.0))).unwrap());
            let _ = lua_canvas.set("measure_text", measure_text);

            let lua_monitor = LuaMonitor(monitor.clone());
            
            if let Ok(measure_fn) = globals.get::<Function>("measure") {
                match measure_fn.call::<(u32, u32)>((lua_canvas, lua_monitor)) {
                    Ok((w, h)) => Ok(Size::new(w, h)),
                    Err(e) => {
                        eprintln!("Lua measure error in {}: {}", self.name, e);
                        Ok(Size::new(0, 0))
                    }
                }
            } else {
                Ok(Size::new(0, 0))
            }
        });
        res.unwrap_or(Size::new(0, 0))
    }

    fn on_event(&mut self, event: crate::domain::events::InputEvent) -> Vec<crate::domain::commands::AppCommand> {
        use crate::domain::commands::AppCommand;
        let mut commands = Vec::new();
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let globals = lua.globals();
        
        if let Ok(on_event_fn) = globals.get::<Function>("on_event") {
            let event_table = match lua.create_table() {
                Ok(t) => t,
                Err(e) => { tracing::error!("Failed to create event table: {}", e); return commands; }
            };
            use crate::domain::events::InputEvent;
            match event {
                InputEvent::PointerEnter => { let _ = event_table.set("type", "pointer_enter"); },
                InputEvent::PointerLeave => { let _ = event_table.set("type", "pointer_leave"); },
                InputEvent::Click { button, x, y } => {
                    let _ = event_table.set("type", "click");
                    let _ = event_table.set("button", button);
                    let _ = event_table.set("x", x);
                    let _ = event_table.set("y", y);
                }
                InputEvent::Scroll { axis, amount } => {
                    let _ = event_table.set("type", "scroll");
                    let _ = event_table.set("axis", axis);
                    let _ = event_table.set("amount", amount);
                }
                InputEvent::MetricsState(state) => {
                    let _ = event_table.set("type", "metrics");
                    if let Ok(val) = lua.to_value(&state) {
                        let _ = event_table.set("metrics", val);
                    }
                }
                InputEvent::AppletsState(state) => {
                    let _ = event_table.set("type", "applets");
                    if let Ok(val) = lua.to_value(&state) {
                        let _ = event_table.set("applets", val);
                    }
                }
                _ => {
                    // Ignore other events for now
                }
            }
            let commands_cell = RefCell::new(&mut commands);
            
            let _ = lua.scope(|scope| {
                let cranky_table = lua.create_table().unwrap();
                let applet_action = scope.create_function(|_, (id, action): (String, String)| {
                    commands_cell.borrow_mut().push(AppCommand::AppletAction { id, action });
                    Ok(())
                }).unwrap();
                let _ = cranky_table.set("applet_action", applet_action);
                let _ = globals.set("cranky", cranky_table);

                let _ = on_event_fn.call::<()>(event_table).unwrap_or_else(|e| {
                    eprintln!("Lua on_event error in {}: {}", self.name, e);
                });
                Ok::<(), mlua::Error>(())
            });
        }
        commands
    }
}
