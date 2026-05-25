use mlua::{Lua, UserData, UserDataMethods, LuaSerdeExt, Function, Value};
use crate::ports::canvas::Canvas;
use crate::domain::signals::{SignalHub, SignalKind};
use crate::domain::config::{ModuleConfig, BarConfig};
use crate::domain::{ModuleId, MonitorId, geometry::Size};
use crate::domain::errors::DomainError;
use crate::modules::AnyModule;
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

impl AnyModule for LuaModule {
    fn init(
        &mut self,
        config: &ModuleConfig,
        bar_config: &BarConfig,
    ) -> Result<(), DomainError> {
        let lua = self.lua.lock().unwrap();
        let globals = lua.globals();
        
        // Expose bar config
        let bar_config_table = lua.create_table().map_err(|e| DomainError::Internal { message: e.to_string() })?;
        bar_config_table.set("font_family", bar_config.font_family().as_str()).map_err(|e| DomainError::Internal { message: e.to_string() })?;
        bar_config_table.set("font_size", bar_config.font_size().value()).map_err(|e| DomainError::Internal { message: e.to_string() })?;
        globals.set("bar_config", bar_config_table).map_err(|e| DomainError::Internal { message: e.to_string() })?;

        // Expose module config options using mlua's serde support
        let options_lua = lua.to_value(config.options())
            .map_err(|e| DomainError::Internal { message: format!("Failed to convert config to Lua: {}", e) })?;
        globals.set("config", options_lua).map_err(|e| DomainError::Internal { message: e.to_string() })?;

        // Load the script
        lua.load(&self.source)
            .set_name(&self.name)
            .exec()
            .map_err(|e| DomainError::Internal { message: format!("Lua load error in {}: {}", self.name, e) })?;

        // Call init if it exists
        if let Ok(init_fn) = globals.get::<Function>("init") {
            init_fn.call::<()>(()).map_err(|e| DomainError::Internal { message: format!("Lua init error in {}: {}", self.name, e) })?;
        }

        Ok(())
    }

    fn subscriptions(&self) -> Vec<SignalKind> {
        let lua = self.lua.lock().unwrap();
        let globals = lua.globals();
        
        let mut subs = Vec::new();
        if let Ok(subs_fn) = globals.get::<Function>("subscriptions") {
            if let Ok(result) = subs_fn.call::<mlua::Table>(()) {
                for pair in result.pairs::<mlua::Value, String>() {
                    if let Ok((_, val)) = pair {
                        match val.as_str() {
                            "time" => subs.push(SignalKind::Time),
                            "hyprland" => subs.push(SignalKind::Hyprland),
                            _ => {}
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
        let lua = self.lua.lock().unwrap();
        let globals = lua.globals();
        
        let time = *hub.time_rx().borrow();
        let _ = globals.set("current_time", time.to_rfc3339());

        let hypr = hub.hyprland_rx().borrow().clone();
        if let Ok(hypr_lua) = lua.to_value(&hypr) {
            let _ = globals.set("hyprland", hypr_lua);
        }
        
        if let Ok(refresh_fn) = globals.get::<Function>("refresh") {
            let _ = refresh_fn.call::<()>(());
        }
    }

    fn view(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) {
        let lua = self.lua.lock().unwrap();
        let canvas_cell = RefCell::new(canvas);
        
        let _ = lua.scope(|scope| {
            let globals = lua.globals();
            let lua_canvas = lua.create_table().unwrap();
            
            let draw_rect = scope.create_function(|_, (_self, x, y, w, h, color_str, radius): (Value, f32, f32, f32, f32, String, f32)| {
                let color = DrawingColor::parse(&color_str).map_err(mlua::Error::external)?;
                canvas_cell.borrow_mut().draw_rect(x, y, w, h, color, radius);
                Ok(())
            }).unwrap();
            lua_canvas.set("draw_rect", draw_rect).unwrap();

            let draw_border = scope.create_function(|_, (_self, x, y, w, h, color_str, radius, size): (Value, f32, f32, f32, f32, String, f32, f32)| {
                let color = DrawingColor::parse(&color_str).map_err(mlua::Error::external)?;
                canvas_cell.borrow_mut().draw_border(x, y, w, h, color, radius, size);
                Ok(())
            }).unwrap();
            lua_canvas.set("draw_border", draw_border).unwrap();

            let draw_text = scope.create_function(|_, (_self, text, font, size, color_str, x, y): (Value, String, String, f32, String, f32, f32)| {
                let color = DrawingColor::parse(&color_str).map_err(mlua::Error::external)?;
                canvas_cell.borrow_mut().draw_text(&text, &font, size, color, x, y);
                Ok(())
            }).unwrap();
            lua_canvas.set("draw_text", draw_text).unwrap();

            let measure_text = scope.create_function(|_, (_self, text, font, size): (Value, String, String, f32)| {
                let (w, h) = canvas_cell.borrow_mut().measure_text(&text, &font, size);
                Ok((w, h))
            }).unwrap();
            lua_canvas.set("measure_text", measure_text).unwrap();

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
        let lua = self.lua.lock().unwrap();
        let canvas_cell = RefCell::new(canvas);
        
        let res = lua.scope(|scope| {
            let globals = lua.globals();
            let lua_canvas = lua.create_table().unwrap();
            
            let measure_text = scope.create_function(|_, (_self, text, font, size): (Value, String, String, f32)| {
                let (w, h) = canvas_cell.borrow_mut().measure_text(&text, &font, size);
                Ok((w, h))
            }).unwrap();
            lua_canvas.set("measure_text", measure_text).unwrap();

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

    fn on_event(&mut self, event: crate::domain::events::InputEvent) {
        let lua = self.lua.lock().unwrap();
        let globals = lua.globals();
        
        if let Ok(on_event_fn) = globals.get::<Function>("on_event") {
            let event_table = lua.create_table().unwrap();
            use crate::domain::events::InputEvent;
            match event {
                InputEvent::PointerEnter => event_table.set("type", "pointer_enter").unwrap(),
                InputEvent::PointerLeave => event_table.set("type", "pointer_leave").unwrap(),
                InputEvent::Click { button, x, y } => {
                    event_table.set("type", "click").unwrap();
                    event_table.set("button", button).unwrap();
                    event_table.set("x", x).unwrap();
                    event_table.set("y", y).unwrap();
                }
                InputEvent::Scroll { axis, amount } => {
                    event_table.set("type", "scroll").unwrap();
                    event_table.set("axis", axis).unwrap();
                    event_table.set("amount", amount).unwrap();
                }
            }
            let _ = on_event_fn.call::<()>(event_table).unwrap_or_else(|e| {
                eprintln!("Lua on_event error in {}: {}", self.name, e);
            });
        }
    }
}
