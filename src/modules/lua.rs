use crate::domain::config::{BarConfig, FontFamily, FontSize, ModuleConfig};
use crate::domain::dbus::{BusType, DBusSubscription};
use crate::domain::shared::color::DrawingColor;
use crate::domain::signals::{SignalHub, SignalKind};
use crate::domain::{
    MonitorId,
    shared::geometry::{LogicalPx, Position, Size},
};
use crate::ports::canvas::Canvas;
use crate::ports::registry::AnyModulePort;
use mlua::{Function, Lua, LuaSerdeExt, UserData, UserDataMethods};
use std::cell::RefCell;
use std::sync::Mutex;

#[derive(Clone)]
pub struct LuaMonitor(pub MonitorId);

impl UserData for LuaMonitor {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| Ok(this.0.as_str().to_string()));
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
    fn init(&mut self, config: &ModuleConfig, bar_config: &BarConfig) -> Result<(), String> {
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let globals = lua.globals();

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
        if let Ok(subs_fn) = globals.get::<Function>("subscriptions") {
            if let Ok(result) = subs_fn.call::<mlua::Value>(())
                && let mlua::Value::Table(t) = result
            {
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
                        && typ == "dbus"
                    {
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
        if let Ok(applets_lua) = lua.to_value(&applets_state.items()) {
            let _ = globals.set("applets", applets_lua);
        }

        let metrics_state = hub.metrics_rx().borrow().clone();
        if let Ok(metrics_lua) = lua.to_value(&metrics_state) {
            let _ = globals.set("metrics", metrics_lua);
        }

        if let Ok(refresh_fn) = globals.get::<Function>("refresh") {
            let _ = refresh_fn.call::<()>(());
        }
    }

    fn view(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) {
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let canvas_cell = RefCell::new(canvas);
        let with_canvas = |f: &mut dyn FnMut(&mut dyn Canvas)| f(*canvas_cell.borrow_mut());

        let _ = lua.scope(|scope| {
            let globals = lua.globals();
            let lua_canvas = match lua.create_table() {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("Failed to create lua canvas table: {}", e);
                    return Ok(());
                }
            };

            let draw_rect = scope
                .create_function(
                    |_,
                     (_self, x, y, w, h, color_str, radius): (
                        mlua::Value,
                        f64,
                        f64,
                        f64,
                        f64,
                        String,
                        Option<f64>,
                    )| {
                        if let Ok(color) = DrawingColor::parse(&color_str) {
                            let r = radius.unwrap_or(0.0);
                            with_canvas(&mut |c| {
                                c.draw_rect(
                                    LogicalPx::new(x as f32),
                                    LogicalPx::new(y as f32),
                                    LogicalPx::new(w as f32),
                                    LogicalPx::new(h as f32),
                                    color.clone(),
                                    LogicalPx::new(r as f32),
                                )
                            });
                        }
                        Ok(())
                    },
                )
                .unwrap_or_else(|_| scope.create_function(|_, ()| Ok(())).unwrap());
            let _ = lua_canvas.set("draw_rect", draw_rect);

            let draw_border = scope
                .create_function(
                    |_,
                     (_self, x, y, w, h, color_str, radius, size): (
                        mlua::Value,
                        f64,
                        f64,
                        f64,
                        f64,
                        String,
                        Option<f64>,
                        Option<f64>,
                    )| {
                        if let Ok(color) = DrawingColor::parse(&color_str) {
                            let r = radius.unwrap_or(0.0);
                            let s = size.unwrap_or(1.0);
                            with_canvas(&mut |c| {
                                c.draw_border(crate::domain::shared::geometry::Position::new(x as i32, y as i32), crate::domain::shared::geometry::Size::new(w as u32, h as u32), color.clone(), LogicalPx::new(r as f32), LogicalPx::new(s as f32))
                            });
                        }
                        Ok(())
                    },
                )
                .unwrap_or_else(|_| scope.create_function(|_, ()| Ok(())).unwrap());
            let _ = lua_canvas.set("draw_border", draw_border);

            let draw_text = scope
                .create_function(
                    |_,
                     (_self, text, color_str, x, y, font, size): (
                        mlua::Value,
                        String,
                        String,
                        f64,
                        f64,
                        Option<String>,
                        Option<f64>,
                    )| {
                        if let Ok(color) = DrawingColor::parse(&color_str) {
                            let font_family = font.map(FontFamily::new);
                            let font_size = size.map(|s| FontSize::new(s as f32));
                            let position = Position::new(x as i32, y as i32);
                            with_canvas(&mut |c| {
                                c.draw_text(
                                    &text,
                                    font_family.as_ref(),
                                    font_size,
                                    color.clone(),
                                    position,
                                )
                            });
                        }
                        Ok(())
                    },
                )
                .unwrap_or_else(|_| scope.create_function(|_, ()| Ok(())).unwrap());
            let _ = lua_canvas.set("draw_text", draw_text);

            let draw_image = scope
                .create_function(
                    |lua,
                     (
                        _self,
                        image_data_val,
                        width,
                        height,
                        logical_width,
                        logical_height,
                        x,
                        y,
                    ): (
                        mlua::Value,
                        mlua::Value,
                        u32,
                        u32,
                        f64,
                        f64,
                        f64,
                        f64,
                    )| {
                        let image_data: Vec<u8> =
                            lua.from_value(image_data_val).unwrap_or_default();
                        with_canvas(&mut |c| {
                            c.draw_image(&image_data, crate::domain::shared::geometry::Size::new(width, height), crate::domain::shared::geometry::Size::new(logical_width as u32, logical_height as u32), crate::domain::shared::geometry::Position::new(x as i32, y as i32))
                        });
                        Ok(())
                    },
                )
                .unwrap_or_else(|_| scope.create_function(|_, ()| Ok(())).unwrap());
            let _ = lua_canvas.set("draw_image", draw_image);

            let measure_text = scope
                .create_function(
                    |_,
                     (_self, text, font, size): (
                        mlua::Value,
                        String,
                        Option<String>,
                        Option<f64>,
                    )| {
                        let mut res = (0.0, 0.0);
                        let font_family = font.map(FontFamily::new);
                        let font_size = size.map(|s| FontSize::new(s as f32));
                        with_canvas(&mut |c| {
                            let (w, h) = c.measure_text(&text, font_family.as_ref(), font_size);
                            res = (w.value() as f64, h.value() as f64);
                        });
                        Ok(res)
                    },
                )
                .unwrap_or_else(|_| scope.create_function(|_, ()| Ok((0.0, 0.0))).unwrap());
            let _ = lua_canvas.set("measure_text", measure_text);

            let lua_monitor = LuaMonitor(monitor.clone());

            if let Ok(view_fn) = globals.get::<Function>("view") {
                view_fn
                    .call::<()>((lua_canvas, lua_monitor))
                    .unwrap_or_else(|e| {
                        eprintln!("Lua view error in {}: {}", self.name, e);
                    });
            }
            Ok(())
        });
    }

    fn measure(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) -> Size {
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let canvas_cell = RefCell::new(canvas);
        let with_canvas = |f: &mut dyn FnMut(&mut dyn Canvas)| f(*canvas_cell.borrow_mut());

        let res = lua.scope(|scope| {
            let globals = lua.globals();
            let lua_canvas = match lua.create_table() {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("Failed to create lua canvas table: {}", e);
                    return Ok(Size::new(0, 0));
                }
            };

            let measure_text = scope
                .create_function(
                    |_,
                     (_self, text, font, size): (
                        mlua::Value,
                        String,
                        Option<String>,
                        Option<f64>,
                    )| {
                        let mut res = (0.0, 0.0);
                        let font_family = font.map(FontFamily::new);
                        let font_size = size.map(|s| FontSize::new(s as f32));
                        with_canvas(&mut |c| {
                            let (w, h) = c.measure_text(&text, font_family.as_ref(), font_size);
                            res = (w.value() as f64, h.value() as f64);
                        });
                        Ok(res)
                    },
                )
                .unwrap_or_else(|_| scope.create_function(|_, ()| Ok((0.0, 0.0))).unwrap());
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

    fn on_pointer_event(
        &mut self,
        event: crate::domain::events::PointerEvent,
    ) -> Vec<crate::domain::commands::AppCommand> {
        use crate::domain::commands::AppCommand;
        let mut commands = Vec::new();
        let lua = self.lua.lock().unwrap_or_else(|e| e.into_inner());
        let globals = lua.globals();

        if let Ok(on_event_fn) = globals.get::<Function>("on_event") {
            let event_table = match lua.create_table() {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("Failed to create event table: {}", e);
                    return commands;
                }
            };
            use crate::domain::events::PointerEvent;
            match event {
                PointerEvent::PointerEnter => {
                    let _ = event_table.set("type", "pointer_enter");
                }
                PointerEvent::PointerLeave => {
                    let _ = event_table.set("type", "pointer_leave");
                }
                PointerEvent::PointerMotion { x, y } => {
                    let _ = event_table.set("type", "motion");
                    let _ = event_table.set("x", x);
                    let _ = event_table.set("y", y);
                }
                PointerEvent::Click { button, x, y } => {
                    let _ = event_table.set("type", "click");
                    let _ = event_table.set("button", button);
                    let _ = event_table.set("x", x);
                    let _ = event_table.set("y", y);
                }
                PointerEvent::Scroll { axis, amount } => {
                    let _ = event_table.set("type", "scroll");
                    let _ = event_table.set("axis", axis);
                    let _ = event_table.set("amount", amount);
                }
            }
            let commands_cell = RefCell::new(&mut commands);

            let _ = lua.scope(|scope| {
                let cranky_table = lua.create_table().unwrap();
                let applet_action = scope
                    .create_function(|_, (id, action): (String, String)| {
                        commands_cell
                            .borrow_mut()
                            .push(AppCommand::AppletAction { id, action });
                        Ok(())
                    })
                    .unwrap();
                let _ = cranky_table.set("applet_action", applet_action);

                let show_tooltip = scope
                    .create_function(|_, text: String| {
                        commands_cell
                            .borrow_mut()
                            .push(AppCommand::ShowTooltip { text });
                        Ok(())
                    })
                    .unwrap();
                let _ = cranky_table.set("show_tooltip", show_tooltip);

                let hide_tooltip = scope
                    .create_function(|_, ()| {
                        commands_cell.borrow_mut().push(AppCommand::HideTooltip);
                        Ok(())
                    })
                    .unwrap();
                let _ = cranky_table.set("hide_tooltip", hide_tooltip);

                let _ = globals.set("cranky", cranky_table);

                on_event_fn.call::<()>(event_table).unwrap_or_else(|e| {
                    eprintln!("Lua on_event error in {}: {}", self.name, e);
                });
                Ok::<(), mlua::Error>(())
            });
        }
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MonitorId;
    use crate::domain::applets::{
        AppletId, AppletItem, AppletStatus, AppletsState, Destination, ObjectPath, Title,
    };
    use crate::domain::config::{BarConfig, ModuleConfig};
    use crate::domain::shared::geometry::LogicalPx;
    use crate::ports::canvas::MockCanvas;
    use std::collections::HashMap;

    #[test]
    fn test_applet_missing_icon_regression() {
        let mut module = LuaModule::built_in("applet").expect("Failed to load applet module");
        let module_config = ModuleConfig::new("applet".into(), true, HashMap::new());
        let bar_config = BarConfig::default();

        module
            .init(&module_config, &bar_config)
            .expect("Init failed");

        let hub = SignalHub::new(crate::domain::config::Config::default());
        let item = AppletItem::new(crate::domain::applets::CreateAppletCommand { id: AppletId::new("test_applet"), destination: Destination::new("dest"), path: ObjectPath::new("/path"), title: Title::new("Test Applet"), status: AppletStatus::Active, icon_name: None, icon_image: None, menu_path: // icon_image is None
            None, });

        hub.applets_tx()
            .send(AppletsState::new(vec![item]))
            .unwrap();

        module.refresh(&hub);

        let mut canvas = MockCanvas::new();
        // The applet module should draw a placeholder rectangle for missing icons.
        // If it successfully renders the placeholder, it will call draw_rect.
        canvas
            .expect_draw_rect()
            .times(1..)
            .returning(|_, _, _, _, _, _| ());

        // Mock measure_text and draw_text to allow the rest of the view logic to succeed
        canvas
            .expect_measure_text()
            .returning(|_, _, _| (LogicalPx::new(10.0), LogicalPx::new(10.0)));
        canvas.expect_draw_text().returning(|_, _, _, _, _| ());

        // Mock draw_image to panic if it's somehow called (it shouldn't be, since there's no icon)
        canvas
            .expect_draw_image()
            .times(0)
            .returning(|_, _, _, _| ());

        module.view(&mut canvas, &MonitorId::new("DP-1"));
    }
}
