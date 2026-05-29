#![allow(unsafe_code)]

use rhai::{Engine, Scope, AST, Dynamic, Array};
use crate::ports::canvas::Canvas;
use crate::domain::signals::{SignalHub, SignalKind};
use crate::domain::config::{ModuleConfig, BarConfig};
use crate::domain::{ModuleId, MonitorId, geometry::Size};
use crate::domain::errors::DomainError;
use crate::modules::AnyModule;
use crate::domain::color::DrawingColor;
use std::sync::Mutex;

#[derive(Copy, Clone)]
struct CanvasPtr(*mut (dyn Canvas + 'static));

thread_local! {
    static CURRENT_CANVAS: std::cell::Cell<Option<CanvasPtr>> = std::cell::Cell::new(None);
}

fn with_canvas<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut dyn Canvas) -> R,
{
    if let Some(ptr) = CURRENT_CANVAS.with(|c| c.get()) {
        Some(f(unsafe { &mut *ptr.0 }))
    } else {
        None
    }
}

pub struct RhaiModule {
    engine: Mutex<Engine>,
    scope: Mutex<Scope<'static>>,
    ast: AST,
    name: String,
}

impl RhaiModule {
    pub fn new(name: String, source: &str) -> Result<Self, DomainError> {
        let mut engine = Engine::new();
        
        // Register API functions once
        engine.register_fn("draw_rect", |x: f32, y: f32, w: f32, h: f32, color_str: String, radius: f32| {
            if let Ok(color) = DrawingColor::parse(&color_str) {
                with_canvas(|c| c.draw_rect(x, y, w, h, color, radius));
            }
        });

        engine.register_fn("draw_border", |x: f32, y: f32, w: f32, h: f32, color_str: String, radius: f32, size: f32| {
            if let Ok(color) = DrawingColor::parse(&color_str) {
                with_canvas(|c| c.draw_border(x, y, w, h, color, radius, size));
            }
        });

        engine.register_fn("draw_text", |text: String, font: String, size: f32, color_str: String, x: f32, y: f32| {
            if let Ok(color) = DrawingColor::parse(&color_str) {
                with_canvas(|c| c.draw_text(&text, &font, size, color, x, y));
            }
        });

        engine.register_fn("measure_text", |text: String, font: String, size: f32| -> Array {
            with_canvas(|c| {
                let (w, h) = c.measure_text(&text, &font, size);
                vec![Dynamic::from(w), Dynamic::from(h)]
            }).unwrap_or_else(|| vec![Dynamic::from(0.0), Dynamic::from(0.0)])
        });

        engine.register_fn("draw_image", |image_data: Vec<u8>, width: i64, height: i64, logical_width: f32, logical_height: f32, x: f32, y: f32| {
            with_canvas(|c| c.draw_image(&image_data, width as u32, height as u32, logical_width, logical_height, x, y));
        });

        engine.register_fn("exec", |cmd: String| {
            let _ = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .spawn();
        });

        let ast = engine.compile(source).map_err(|e| DomainError::Internal { 
            message: format!("Failed to compile Rhai script {}: {}", name, e) 
        })?;
        
        Ok(Self {
            engine: Mutex::new(engine),
            scope: Mutex::new(Scope::new()),
            ast,
            name,
        })
    }

    pub fn built_in(name: &str) -> Option<Self> {
        let source = match name {
            "hour" => include_str!("builtins/hour.rhai"),
            "workspace" => include_str!("builtins/workspace.rhai"),
            _ => return None,
        };
        Self::new(name.to_string(), source).ok()
    }

    pub fn external(name: &str) -> Option<Self> {
        let home = std::env::var("HOME").ok()?;
        let path = std::path::PathBuf::from(home)
            .join(".config/cranky/modules")
            .join(format!("{}.rhai", name));
        
        if path.exists() {
            let source = std::fs::read_to_string(path).ok()?;
            Self::new(name.to_string(), &source).ok()
        } else {
            None
        }
    }
}

impl AnyModule for RhaiModule {
    fn init(
        &mut self,
        config: &ModuleConfig,
        bar_config: &BarConfig,
    ) -> Result<(), DomainError> {
        let mut scope = self.scope.lock().unwrap_or_else(|e| e.into_inner());
        let engine = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        
        // Expose bar config
        let mut bar_map = rhai::Map::new();
        bar_map.insert("font_family".into(), Dynamic::from(bar_config.font_family().as_str().to_string()));
        bar_map.insert("font_size".into(), Dynamic::from(bar_config.font_size().value()));
        scope.push_constant("bar_config", bar_map);

        // Expose module config options
        let options_json = serde_json::to_string(config.options())
            .map_err(|e| DomainError::Internal { message: e.to_string() })?;
        let options_rhai: rhai::Map = engine.parse_json(&options_json, true)
            .map_err(|e| DomainError::Internal { message: e.to_string() })?;
        scope.push_constant("config", options_rhai);

        // Call init if it exists
        let _ = engine.call_fn::<()>(&mut scope, &self.ast, "init", ());
        Ok(())
    }

    fn subscriptions(&self) -> Vec<SignalKind> {
        let mut scope = self.scope.lock().unwrap_or_else(|e| e.into_inner());
        let engine = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        
        let mut subs = Vec::new();
        if let Ok(result) = engine.call_fn::<Array>(&mut scope, &self.ast, "subscriptions", ()) {
            for val in result {
                if let Ok(s) = val.into_string() {
                    match s.as_str() {
                        "time" => subs.push(SignalKind::Time),
                        "hyprland" => subs.push(SignalKind::Hyprland),
                        _ => {}
                    }
                }
            }
        }
        subs
    }

    fn refresh(&mut self, hub: &SignalHub) {
        let mut scope = self.scope.lock().unwrap_or_else(|e| e.into_inner());
        let engine = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        
        let time = *hub.time_rx().borrow();
        scope.set_or_push("current_time", time.to_rfc3339());

        let hypr = hub.hyprland_rx().borrow().clone();
        if let Ok(hypr_json) = serde_json::to_string(&hypr) {
            if let Ok(hypr_rhai) = engine.parse_json(&hypr_json, true) {
                scope.set_or_push("hyprland", hypr_rhai);
            }
        }
        
        let _ = engine.call_fn::<()>(&mut scope, &self.ast, "refresh", ());
    }

    fn view(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) {
        let mut scope = self.scope.lock().unwrap_or_else(|e| e.into_inner());
        let engine = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        
        let ptr = unsafe { std::mem::transmute::<*mut dyn Canvas, *mut (dyn Canvas + 'static)>(canvas as *mut dyn Canvas) };
        CURRENT_CANVAS.with(|c| c.set(Some(CanvasPtr(ptr))));

        let monitor_id = monitor.as_str().to_string();
        let _ = engine.call_fn::<()>(&mut scope, &self.ast, "view", (monitor_id,));
        
        CURRENT_CANVAS.with(|c| c.set(None));
    }

    fn measure(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) -> Size {
        let mut scope = self.scope.lock().unwrap_or_else(|e| e.into_inner());
        let engine = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        
        let ptr = unsafe { std::mem::transmute::<*mut dyn Canvas, *mut (dyn Canvas + 'static)>(canvas as *mut dyn Canvas) };
        CURRENT_CANVAS.with(|c| c.set(Some(CanvasPtr(ptr))));

        let monitor_id = monitor.as_str().to_string();
        let size = if let Ok(result) = engine.call_fn::<Array>(&mut scope, &self.ast, "measure", (monitor_id,)) {
            if result.len() == 2 {
                let w = result[0].as_int().unwrap_or(0) as u32;
                let h = result[1].as_int().unwrap_or(0) as u32;
                Size::new(w, h)
            } else {
                Size::new(0, 0)
            }
        } else {
            Size::new(0, 0)
        };

        CURRENT_CANVAS.with(|c| c.set(None));
        size
    }

    fn on_event(&mut self, event: crate::domain::events::InputEvent) -> Vec<crate::domain::commands::AppCommand> {
        let mut commands = Vec::new();
        let mut scope = self.scope.lock().unwrap_or_else(|e| e.into_inner());
        let engine = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        
        let mut event_map = rhai::Map::new();
        use crate::domain::events::InputEvent;
        match event {
            InputEvent::PointerEnter => {
                event_map.insert("type".into(), Dynamic::from("pointer_enter".to_string()));
            }
            InputEvent::PointerLeave => {
                event_map.insert("type".into(), Dynamic::from("pointer_leave".to_string()));
            }
            InputEvent::Click { button, x, y } => {
                event_map.insert("type".into(), Dynamic::from("click".to_string()));
                event_map.insert("button".into(), Dynamic::from(button as i64));
                event_map.insert("x".into(), Dynamic::from(x));
                event_map.insert("y".into(), Dynamic::from(y));
            }
            InputEvent::Scroll { axis, amount } => {
                event_map.insert("type".into(), Dynamic::from("scroll".to_string()));
                event_map.insert("axis".into(), Dynamic::from(axis as i64));
                event_map.insert("amount".into(), Dynamic::from(amount));
            }
            InputEvent::MetricsState(_) => {
                event_map.insert("type".into(), Dynamic::from("metrics".to_string()));
                // Serialization of full MetricsState not implemented for Rhai yet
            }
            InputEvent::AppletsState(_) => {
                event_map.insert("type".into(), Dynamic::from("applets".to_string()));
            }
            _ => {}
        }
        let _ = engine.call_fn::<()>(&mut scope, &self.ast, "on_event", (event_map,));
        commands
    }
}
