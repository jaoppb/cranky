use crate::core::hyprland::{HyprlandProvider, RealHyprlandProvider, Workspace};
use crate::modules::{CrankyModule, Event, UpdateAction};
use crate::render::{RenderContext, TextStyling};
use crate::utils::parse_color;
use log::error;
use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, PixmapMut, Rect, Transform};

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("Hyprland error: {0}")]
    Hyprland(#[from] crate::core::hyprland::HyprError),
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkspaceConfig {
    #[serde(default)]
    active: ActiveWorkspaceConfig,
    #[serde(default)]
    border_radius: f32,
}

impl WorkspaceConfig {
    pub fn active(&self) -> &ActiveWorkspaceConfig {
        &self.active
    }

    pub fn border_radius(&self) -> f32 {
        self.border_radius
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            active: ActiveWorkspaceConfig::default(),
            border_radius: 0.0,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ActiveWorkspaceConfig {
    #[serde(default = "default_active_bg")]
    background_color: String,
}

impl ActiveWorkspaceConfig {
    pub fn background_color(&self) -> &str {
        &self.background_color
    }
}

impl Default for ActiveWorkspaceConfig {
    fn default() -> Self {
        Self {
            background_color: default_active_bg(),
        }
    }
}

fn default_active_bg() -> String {
    "#3b4261".to_string()
}

pub struct WorkspaceModule {
    provider: Box<dyn HyprlandProvider>,
    workspaces: Vec<Workspace>,
    active_workspaces: HashMap<String, i32>,
    font_family: String,
    active_background: Color,
    border_radius: f32,
}

impl WorkspaceModule {
    pub fn new() -> Self {
        Self {
            provider: Box::new(RealHyprlandProvider),
            workspaces: Vec::new(),
            active_workspaces: HashMap::new(),
            font_family: String::new(),
            active_background: Color::from_rgba8(59, 66, 97, 255),
            border_radius: 0.0,
        }
    }

    pub fn with_provider(provider: Box<dyn HyprlandProvider>) -> Self {
        Self {
            provider,
            workspaces: Vec::new(),
            active_workspaces: HashMap::new(),
            font_family: String::new(),
            active_background: Color::from_rgba8(59, 66, 97, 255),
            border_radius: 0.0,
        }
    }

    fn fill_rounded_rect(&self, pixmap: &mut PixmapMut, rect: Rect, radius: f32, color: Color) {
        let x = rect.left();
        let y = rect.top();
        let w = rect.width();
        let h = rect.height();
        let r = radius.min(w / 2.0).min(h / 2.0);

        if r <= 0.0 {
            let path = PathBuilder::from_rect(rect);
            let mut paint = Paint::default();
            paint.set_color(color);
            paint.anti_alias = true;
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
            return;
        }

        let mut pb = PathBuilder::new();
        pb.move_to(x + r, y);
        pb.line_to(x + w - r, y);
        pb.quad_to(x + w, y, x + w, y + r);
        pb.line_to(x + w, y + h - r);
        pb.quad_to(x + w, y + h, x + w - r, y + h);
        pb.line_to(x + r, y + h);
        pb.quad_to(x, y + h, x, y + h - r);
        pb.line_to(x, y + r);
        pb.quad_to(x, y, x + r, y);
        pb.close();

        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(color);
            paint.anti_alias = true;
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }
}

impl CrankyModule for WorkspaceModule {
    type Error = WorkspaceError;
    type Config = WorkspaceConfig;

    fn init(
        &mut self,
        config: Self::Config,
        _bar_config: &crate::config::BarConfig,
    ) -> Result<(), Self::Error> {
        self.active_background = parse_color(config.active().background_color());
        self.border_radius = config.border_radius();

        match self.provider.get_workspaces() {
            Ok(mut ws) => {
                ws.sort_by_key(|w| w.id());
                self.workspaces = ws;
            }
            Err(e) => {
                error!("Failed to get initial workspaces: {}", e);
                self.workspaces = Vec::new();
            }
        }

        match self.provider.get_monitors() {
            Ok(monitors) => {
                for m in monitors {
                    self.active_workspaces
                        .insert(m.name().to_string(), m.active_workspace_id());
                }
            }
            Err(e) => {
                error!("Failed to get initial monitors: {}", e);
            }
        }

        Ok(())
    }

    fn update(&mut self, _event: Event) -> UpdateAction {
        let mut redraw = false;

        match self.provider.get_workspaces() {
            Ok(mut new_workspaces) => {
                new_workspaces.sort_by_key(|w| w.id());
                if new_workspaces.len() != self.workspaces.len()
                    || new_workspaces
                        .iter()
                        .zip(&self.workspaces)
                        .any(|(a, b)| a.id() != b.id())
                {
                    self.workspaces = new_workspaces;
                    redraw = true;
                }
            }
            Err(e) => {
                error!("Failed to update workspaces: {}", e);
            }
        }

        match self.provider.get_monitors() {
            Ok(monitors) => {
                for m in monitors {
                    let old_id = self.active_workspaces.get(m.name()).cloned().unwrap_or(-1);
                    if old_id != m.active_workspace_id() {
                        self.active_workspaces
                            .insert(m.name().to_string(), m.active_workspace_id());
                        redraw = true;
                    }
                }
            }
            Err(e) => {
                error!("Failed to update monitors: {}", e);
            }
        }

        if redraw {
            UpdateAction::Redraw
        } else {
            UpdateAction::None
        }
    }

    fn view(&self, pixmap: &mut PixmapMut, area: Rect, context: &mut RenderContext, monitor: &str) {
        let monitor_workspaces: Vec<&Workspace> = self
            .workspaces
            .iter()
            .filter(|w| w.monitor() == monitor)
            .collect();

        let active_id = self.active_workspaces.get(monitor).cloned().unwrap_or(-1);
        let scale = context.scale();

        let styling = TextStyling::new(
            14.0,
            20.0,
            Color::from_rgba8(122, 162, 247, 255),
            self.font_family.clone(),
        );

        let active_styling = TextStyling::new(
            14.0,
            20.0,
            Color::from_rgba8(255, 255, 255, 255),
            self.font_family.clone(),
        );

        let y_offset = context.calculate_vertical_offset(area, styling.line_height());

        // Use scaled units for layout
        let item_size = 24.0;
        let item_spacing = 30.0;

        let mut x_offset = area.left();
        for ws in monitor_workspaces {
            let label = ws.id().to_string();
            let is_active = ws.id() == active_id;

            if is_active {
                // Background rectangle (already in physical pixels because we scale the logical coordinates)
                let bg_x = x_offset * scale;
                let bg_y = (area.top() + (area.height() - item_size) / 2.0) * scale;
                let bg_w = item_size * scale;
                let bg_h = item_size * scale;

                if let Some(bg_rect) = Rect::from_xywh(bg_x, bg_y, bg_w, bg_h) {
                    self.fill_rounded_rect(
                        pixmap,
                        bg_rect,
                        self.border_radius * scale,
                        self.active_background,
                    );
                }

                let label_width = context.measure_text(&label, active_styling.clone());
                context.render_text(
                    pixmap,
                    &label,
                    active_styling.clone(),
                    x_offset + (item_size - label_width) / 2.0,
                    y_offset,
                );
            } else {
                let label_width = context.measure_text(&label, styling.clone());
                context.render_text(
                    pixmap,
                    &label,
                    styling.clone(),
                    x_offset + (item_size - label_width) / 2.0,
                    y_offset,
                );
            }
            x_offset += item_spacing;
        }
    }

    fn measure(&self, _context: &mut RenderContext, monitor: &str) -> f32 {
        let monitor_workspaces: Vec<&Workspace> = self
            .workspaces
            .iter()
            .filter(|w| w.monitor() == monitor)
            .collect();

        let item_spacing = 30.0;
        let mut total_width = 0.0;
        for _ws in monitor_workspaces {
            total_width += item_spacing;
        }
        total_width
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_pixmap_has_color;
    use crate::config::BarConfig;
    use crate::core::hyprland::{MockHyprlandProvider, Monitor, Workspace};

    #[test]
    fn test_workspace_init() {
        let mut mock = MockHyprlandProvider::new();
        mock.expect_get_workspaces().returning(|| {
            Ok(vec![
                Workspace::new(1, "eDP-1".to_string()),
                Workspace::new(2, "eDP-1".to_string()),
            ])
        });
        mock.expect_get_monitors().returning(|| {
            Ok(vec![Monitor::new("eDP-1".to_string(), 1)])
        });

        let mut module = WorkspaceModule::with_provider(Box::new(mock));
        let config = WorkspaceConfig::default();
        let bar_config = BarConfig::default();

        module.init(config, &bar_config).unwrap();

        assert_eq!(module.workspaces.len(), 2);
        assert_eq!(module.active_workspaces.get("eDP-1"), Some(&1));
    }

    #[test]
    fn test_workspace_update() {
        let mut mock = MockHyprlandProvider::new();

        // 1st calls (init)
        mock.expect_get_workspaces()
            .times(1)
            .returning(|| Ok(vec![Workspace::new(1, "eDP-1".to_string())]));
        mock.expect_get_monitors()
            .times(1)
            .returning(|| Ok(vec![Monitor::new("eDP-1".to_string(), 1)]));

        // 2nd calls (update) - change active workspace
        mock.expect_get_workspaces()
            .times(1)
            .returning(|| Ok(vec![Workspace::new(1, "eDP-1".to_string())]));
        mock.expect_get_monitors()
            .times(1)
            .returning(|| Ok(vec![Monitor::new("eDP-1".to_string(), 2)]));

        let mut module = WorkspaceModule::with_provider(Box::new(mock));
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        let action = module.update(Event::Timer);
        assert_eq!(action, UpdateAction::Redraw);
        assert_eq!(module.active_workspaces.get("eDP-1"), Some(&2));
    }

    #[test]
    fn test_workspace_measure() {
        let mut mock = MockHyprlandProvider::new();
        mock.expect_get_workspaces()
            .returning(|| Ok(vec![Workspace::new(1, "eDP-1".to_string())]));
        mock.expect_get_monitors()
            .returning(|| Ok(vec![Monitor::new("eDP-1".to_string(), 1)]));

        let mut module = WorkspaceModule::with_provider(Box::new(mock));
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        let mut context = RenderContext::new();
        let width = module.measure(&mut context, "eDP-1");
        assert_eq!(width, 30.0); // 1 workspace * item_spacing(30.0)
    }

    #[test]
    fn test_workspace_view() {
        let mut mock = MockHyprlandProvider::new();
        mock.expect_get_workspaces().returning(|| {
            Ok(vec![
                Workspace::new(1, "eDP-1".to_string()),
                Workspace::new(2, "eDP-1".to_string()),
            ])
        });
        mock.expect_get_monitors()
            .returning(|| Ok(vec![Monitor::new("eDP-1".to_string(), 1)]));

        let mut module = WorkspaceModule::with_provider(Box::new(mock));
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        let mut pixmap_data = vec![0; 100 * 30 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 100, 30).unwrap();
        let mut context = RenderContext::new();
        let area = Rect::from_xywh(0.0, 0.0, 100.0, 30.0).unwrap();

        module.view(&mut pixmap, area, &mut context, "eDP-1");
        
        // Active background color should be present (default: #3b4261 -> RGB(59, 66, 97))
        let expected_color = tiny_skia::Color::from_rgba8(59, 66, 97, 255);
        assert_pixmap_has_color!(pixmap, expected_color);
    }

    #[test]
    fn test_workspace_update_no_change() {
        let mut mock = MockHyprlandProvider::new();
        mock.expect_get_workspaces()
            .returning(|| Ok(vec![Workspace::new(1, "eDP-1".to_string())]));
        mock.expect_get_monitors()
            .returning(|| Ok(vec![Monitor::new("eDP-1".to_string(), 1)]));

        let mut module = WorkspaceModule::with_provider(Box::new(mock));
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        let action = module.update(Event::Timer);
        assert_eq!(action, UpdateAction::None);
    }

    #[test]
    fn test_workspace_init_error() {
        let mut mock = MockHyprlandProvider::new();
        mock.expect_get_workspaces()
            .returning(|| Err(crate::core::hyprland::HyprError::NoInstance));
        mock.expect_get_monitors()
            .returning(|| Err(crate::core::hyprland::HyprError::NoInstance));

        let mut module = WorkspaceModule::with_provider(Box::new(mock));
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        assert_eq!(module.workspaces.len(), 0);
    }

    #[test]
    fn test_workspace_update_error() {
        let mut mock = MockHyprlandProvider::new();
        mock.expect_get_workspaces()
            .times(1)
            .returning(|| Ok(vec![Workspace::new(1, "eDP-1".to_string())]));
        mock.expect_get_monitors()
            .times(1)
            .returning(|| Ok(vec![Monitor::new("eDP-1".to_string(), 1)]));

        mock.expect_get_workspaces()
            .times(1)
            .returning(|| Err(crate::core::hyprland::HyprError::NoInstance));
        mock.expect_get_monitors()
            .times(1)
            .returning(|| Err(crate::core::hyprland::HyprError::NoInstance));

        let mut module = WorkspaceModule::with_provider(Box::new(mock));
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        let action = module.update(Event::Timer);
        assert_eq!(action, UpdateAction::None);
    }

    #[test]
    fn test_workspace_new() {
        // This test will use the real provider, but we just want to exercise the code path.
        // It might fail if HYPRLAND_INSTANCE_SIGNATURE is not set, but we don't care about the result here,
        // just that it doesn't panic and the code is covered.
        let _module = WorkspaceModule::new();
    }

    #[test]
    fn test_workspace_view_rounded() {
        let mut mock = MockHyprlandProvider::new();
        mock.expect_get_workspaces().returning(|| {
            Ok(vec![Workspace::new(1, "eDP-1".to_string())])
        });
        mock.expect_get_monitors()
            .returning(|| Ok(vec![Monitor::new("eDP-1".to_string(), 1)]));

        let mut module = WorkspaceModule::with_provider(Box::new(mock));
        let config = WorkspaceConfig {
            active: ActiveWorkspaceConfig::default(),
            border_radius: 4.0,
        };
        module.init(config, &BarConfig::default()).unwrap();

        let mut pixmap_data = vec![0; 100 * 30 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 100, 30).unwrap();
        let mut context = RenderContext::new();
        let area = Rect::from_xywh(0.0, 0.0, 100.0, 30.0).unwrap();

        module.view(&mut pixmap, area, &mut context, "eDP-1");
        
        let expected_color = tiny_skia::Color::from_rgba8(59, 66, 97, 255);
        assert_pixmap_has_color!(pixmap, expected_color);
    }

    #[test]
    fn test_workspace_update_len_change() {
        let mut mock = MockHyprlandProvider::new();
        mock.expect_get_workspaces()
            .times(1)
            .returning(|| Ok(vec![Workspace::new(1, "eDP-1".to_string())]));
        mock.expect_get_monitors()
            .times(1)
            .returning(|| Ok(vec![Monitor::new("eDP-1".to_string(), 1)]));

        mock.expect_get_workspaces().times(1).returning(|| {
            Ok(vec![
                Workspace::new(1, "eDP-1".to_string()),
                Workspace::new(2, "eDP-1".to_string()),
            ])
        });
        mock.expect_get_monitors()
            .times(1)
            .returning(|| Ok(vec![Monitor::new("eDP-1".to_string(), 1)]));

        let mut module = WorkspaceModule::with_provider(Box::new(mock));
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        let action = module.update(Event::Timer);
        assert_eq!(action, UpdateAction::Redraw);
        assert_eq!(module.workspaces.len(), 2);
    }

    #[test]
    fn test_workspace_update_id_change() {
        let mut mock = MockHyprlandProvider::new();
        mock.expect_get_workspaces()
            .times(1)
            .returning(|| Ok(vec![Workspace::new(1, "eDP-1".to_string())]));
        mock.expect_get_monitors()
            .times(1)
            .returning(|| Ok(vec![Monitor::new("eDP-1".to_string(), 1)]));

        mock.expect_get_workspaces()
            .times(1)
            .returning(|| Ok(vec![Workspace::new(2, "eDP-1".to_string())]));
        mock.expect_get_monitors()
            .times(1)
            .returning(|| Ok(vec![Monitor::new("eDP-1".to_string(), 1)]));

        let mut module = WorkspaceModule::with_provider(Box::new(mock));
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        let action = module.update(Event::Timer);
        assert_eq!(action, UpdateAction::Redraw);
        assert_eq!(module.workspaces[0].id(), 2);
    }
}
