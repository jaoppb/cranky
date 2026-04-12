use crate::core::hyprland::{Workspace};
use crate::modules::{CrankyModule, Event, UpdateAction};
use crate::render::{RenderContext, TextStyling};
use crate::utils::ParsedColor;
use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;
use tiny_skia::{Color, FillRule, PathBuilder, PixmapMut, Rect, Transform};

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
    focused: FocusedWorkspaceConfig,
    #[serde(default)]
    border_radius: f32,
}

impl WorkspaceConfig {
    pub fn active(&self) -> &ActiveWorkspaceConfig {
        &self.active
    }

    pub fn focused(&self) -> &FocusedWorkspaceConfig {
        &self.focused
    }

    pub fn border_radius(&self) -> f32 {
        self.border_radius
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            active: ActiveWorkspaceConfig::default(),
            focused: FocusedWorkspaceConfig::default(),
            border_radius: 0.0,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ActiveWorkspaceConfig {
    #[serde(default = "default_active_bg")]
    background_color: ParsedColor,
}

impl ActiveWorkspaceConfig {
    pub fn background_color(&self) -> &ParsedColor {
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

fn default_active_bg() -> ParsedColor {
    ParsedColor::try_from("#565f89").unwrap()
}

#[derive(Debug, Deserialize, Clone)]
pub struct FocusedWorkspaceConfig {
    #[serde(default = "default_focused_bg")]
    background_color: ParsedColor,
}

impl FocusedWorkspaceConfig {
    pub fn background_color(&self) -> &ParsedColor {
        &self.background_color
    }
}

impl Default for FocusedWorkspaceConfig {
    fn default() -> Self {
        Self {
            background_color: default_focused_bg(),
        }
    }
}

fn default_focused_bg() -> ParsedColor {
    ParsedColor::try_from("#3b4261").unwrap()
}

pub struct WorkspaceModule {
    workspaces: Vec<Workspace>,
    active_workspaces: HashMap<String, i32>,
    focused_monitor: String,
    font_family: String,
    active_background: ParsedColor,
    focused_background: ParsedColor,
    border_radius: f32,
}

impl WorkspaceModule {
    pub fn new() -> Self {
        Self {
            workspaces: Vec::new(),
            active_workspaces: HashMap::new(),
            focused_monitor: String::new(),
            font_family: String::new(),
            active_background: default_active_bg(),
            focused_background: default_focused_bg(),
            border_radius: 0.0,
        }
    }

    fn fill_rounded_rect(&self, pixmap: &mut PixmapMut, rect: Rect, radius: f32, color: &ParsedColor) {
        let x = rect.left();
        let y = rect.top();
        let w = rect.width();
        let h = rect.height();
        let r = radius.min(w / 2.0).min(h / 2.0);

        let paint = color.to_paint(rect);

        if r <= 0.0 {
            let path = PathBuilder::from_rect(rect);
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
        self.active_background = config.active().background_color().clone();
        self.focused_background = config.focused().background_color().clone();
        self.border_radius = config.border_radius();
        Ok(())
    }

    fn update(&mut self, event: Event) -> UpdateAction {
        match event {
            Event::HyprlandUpdate {
                workspaces,
                monitors,
            } => {
                let mut redraw = false;

                let mut new_workspaces = workspaces;
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

                for m in &monitors {
                    let old_id = self.active_workspaces.get(m.name()).cloned().unwrap_or(-1);
                    if old_id != m.active_workspace_id() {
                        self.active_workspaces
                            .insert(m.name().to_string(), m.active_workspace_id());
                        redraw = true;
                    }
                }

                let new_focused = monitors
                    .iter()
                    .find(|m| m.focused())
                    .map(|m| m.name().to_string())
                    .unwrap_or_default();

                if new_focused != self.focused_monitor {
                    self.focused_monitor = new_focused;
                    redraw = true;
                }

                if redraw {
                    UpdateAction::Redraw
                } else {
                    UpdateAction::None
                }
            }
            Event::Timer => UpdateAction::None,
        }
    }

    fn view(&self, pixmap: &mut PixmapMut, area: Rect, context: &mut RenderContext, monitor: &str) {
        let monitor_workspaces: Vec<&Workspace> = self
            .workspaces
            .iter()
            .filter(|w| w.monitor() == monitor)
            .collect();

        let active_id = self.active_workspaces.get(monitor).cloned().unwrap_or(-1);
        let is_monitor_focused = self.focused_monitor == monitor;
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
            let is_visible = ws.id() == active_id;

            if is_visible {
                // Background rectangle (already in physical pixels because we scale the logical coordinates)
                let bg_x = x_offset * scale;
                let bg_y = (area.top() + (area.height() - item_size) / 2.0) * scale;
                let bg_w = item_size * scale;
                let bg_h = item_size * scale;

                let background_color = if is_monitor_focused {
                    &self.active_background
                } else {
                    &self.focused_background
                };

                if let Some(bg_rect) = Rect::from_xywh(bg_x, bg_y, bg_w, bg_h) {
                    self.fill_rounded_rect(
                        pixmap,
                        bg_rect,
                        self.border_radius * scale,
                        background_color,
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
    use crate::core::hyprland::{Monitor, Workspace};

    #[test]
    fn test_workspace_init() {
        let mut module = WorkspaceModule::new();
        let config = WorkspaceConfig::default();
        let bar_config = BarConfig::default();

        module.init(config, &bar_config).unwrap();

        let event = Event::HyprlandUpdate {
            workspaces: vec![
                Workspace::new(1, "eDP-1".to_string()),
                Workspace::new(2, "eDP-1".to_string()),
            ],
            monitors: vec![Monitor::new("eDP-1".to_string(), 1, true)],
        };
        module.update(event);

        assert_eq!(module.workspaces.len(), 2);
        assert_eq!(module.active_workspaces.get("eDP-1"), Some(&1));
        assert_eq!(module.focused_monitor, "eDP-1");
    }

    #[test]
    fn test_workspace_update() {
        let mut module = WorkspaceModule::new();
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        // 1st update
        module.update(Event::HyprlandUpdate {
            workspaces: vec![Workspace::new(1, "eDP-1".to_string())],
            monitors: vec![Monitor::new("eDP-1".to_string(), 1, true)],
        });

        // 2nd update - change active workspace
        let action = module.update(Event::HyprlandUpdate {
            workspaces: vec![Workspace::new(1, "eDP-1".to_string())],
            monitors: vec![Monitor::new("eDP-1".to_string(), 2, true)],
        });

        assert_eq!(action, UpdateAction::Redraw);
        assert_eq!(module.active_workspaces.get("eDP-1"), Some(&2));
    }

    #[test]
    fn test_workspace_measure() {
        let mut module = WorkspaceModule::new();
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        module.update(Event::HyprlandUpdate {
            workspaces: vec![Workspace::new(1, "eDP-1".to_string())],
            monitors: vec![Monitor::new("eDP-1".to_string(), 1, true)],
        });

        let mut context = RenderContext::new();
        let width = module.measure(&mut context, "eDP-1");
        assert_eq!(width, 30.0); // 1 workspace * item_spacing(30.0)
    }

    #[test]
    fn test_workspace_view() {
        let mut module = WorkspaceModule::new();
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        module.update(Event::HyprlandUpdate {
            workspaces: vec![
                Workspace::new(1, "eDP-1".to_string()),
                Workspace::new(2, "eDP-1".to_string()),
            ],
            monitors: vec![Monitor::new("eDP-1".to_string(), 1, true)],
        });

        let mut pixmap_data = vec![0; 100 * 30 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 100, 30).unwrap();
        let mut context = RenderContext::new();
        let area = Rect::from_xywh(0.0, 0.0, 100.0, 30.0).unwrap();

        module.view(&mut pixmap, area, &mut context, "eDP-1");

        // monitor "eDP-1" IS focused (from init), so it should have active_background
        // default active_background: #565f89 -> RGB(86, 95, 137)
        let expected_color = tiny_skia::Color::from_rgba8(86, 95, 137, 255);
        assert_pixmap_has_color!(pixmap, expected_color);
    }

    #[test]
    fn test_workspace_view_focused_on_other_monitor() {
        let mut module = WorkspaceModule::new();
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        module.update(Event::HyprlandUpdate {
            workspaces: vec![
                Workspace::new(1, "eDP-1".to_string()),
                Workspace::new(2, "HDMI-A-1".to_string()),
            ],
            monitors: vec![
                Monitor::new("eDP-1".to_string(), 1, true),
                Monitor::new("HDMI-A-1".to_string(), 2, false),
            ],
        });

        let mut pixmap_data = vec![0; 100 * 30 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 100, 30).unwrap();
        let mut context = RenderContext::new();
        let area = Rect::from_xywh(0.0, 0.0, 100.0, 30.0).unwrap();

        // View HDMI-A-1 (which is NOT focused)
        module.view(&mut pixmap, area, &mut context, "HDMI-A-1");

        // monitor "HDMI-A-1" IS NOT focused, so it should have focused_background
        // default focused_background: #3b4261 -> RGB(59, 66, 97)
        let expected_color = tiny_skia::Color::from_rgba8(59, 66, 97, 255);
        assert_pixmap_has_color!(pixmap, expected_color);
    }

    #[test]
    fn test_workspace_update_no_change() {
        let mut module = WorkspaceModule::new();
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        let event = Event::HyprlandUpdate {
            workspaces: vec![Workspace::new(1, "eDP-1".to_string())],
            monitors: vec![Monitor::new("eDP-1".to_string(), 1, true)],
        };
        module.update(event.clone());

        let action = module.update(event);
        assert_eq!(action, UpdateAction::None);
    }

    #[test]
    fn test_workspace_new() {
        let _module = WorkspaceModule::new();
    }

    #[test]
    fn test_workspace_view_rounded() {
        let mut module = WorkspaceModule::new();
        let config = WorkspaceConfig {
            active: ActiveWorkspaceConfig::default(),
            focused: FocusedWorkspaceConfig::default(),
            border_radius: 4.0,
        };
        module.init(config, &BarConfig::default()).unwrap();

        module.update(Event::HyprlandUpdate {
            workspaces: vec![Workspace::new(1, "eDP-1".to_string())],
            monitors: vec![Monitor::new("eDP-1".to_string(), 1, true)],
        });

        let mut pixmap_data = vec![0; 100 * 30 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 100, 30).unwrap();
        let mut context = RenderContext::new();
        let area = Rect::from_xywh(0.0, 0.0, 100.0, 30.0).unwrap();

        module.view(&mut pixmap, area, &mut context, "eDP-1");

        // monitor "eDP-1" IS focused (from init), so it should have active_background
        // default active_background: #565f89 -> RGB(86, 95, 137)
        let expected_color = tiny_skia::Color::from_rgba8(86, 95, 137, 255);
        assert_pixmap_has_color!(pixmap, expected_color);
    }

    #[test]
    fn test_workspace_update_len_change() {
        let mut module = WorkspaceModule::new();
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        module.update(Event::HyprlandUpdate {
            workspaces: vec![Workspace::new(1, "eDP-1".to_string())],
            monitors: vec![Monitor::new("eDP-1".to_string(), 1, true)],
        });

        let action = module.update(Event::HyprlandUpdate {
            workspaces: vec![
                Workspace::new(1, "eDP-1".to_string()),
                Workspace::new(2, "eDP-1".to_string()),
            ],
            monitors: vec![Monitor::new("eDP-1".to_string(), 1, true)],
        });

        assert_eq!(action, UpdateAction::Redraw);
        assert_eq!(module.workspaces.len(), 2);
    }

    #[test]
    fn test_workspace_update_id_change() {
        let mut module = WorkspaceModule::new();
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        module.update(Event::HyprlandUpdate {
            workspaces: vec![Workspace::new(1, "eDP-1".to_string())],
            monitors: vec![Monitor::new("eDP-1".to_string(), 1, true)],
        });

        let action = module.update(Event::HyprlandUpdate {
            workspaces: vec![Workspace::new(2, "eDP-1".to_string())],
            monitors: vec![Monitor::new("eDP-1".to_string(), 1, true)],
        });

        assert_eq!(action, UpdateAction::Redraw);
        assert_eq!(module.workspaces[0].id(), 2);
    }

    #[test]
    fn test_workspace_config_deserialization() {
        let json = r##"{
            "active": { "background_color": "#ff0000" },
            "focused": { "background_color": "#00ff00" },
            "border_radius": 5.0
        }"##;
        let config: WorkspaceConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.active().background_color(),
            &ParsedColor::try_from("#ff0000").unwrap()
        );
        assert_eq!(
            config.focused().background_color(),
            &ParsedColor::try_from("#00ff00").unwrap()
        );
        assert_eq!(config.border_radius(), 5.0);
    }

    #[test]
    fn test_workspace_update_focus_change() {
        let mut module = WorkspaceModule::new();
        module
            .init(WorkspaceConfig::default(), &BarConfig::default())
            .unwrap();

        module.update(Event::HyprlandUpdate {
            workspaces: vec![Workspace::new(1, "eDP-1".to_string())],
            monitors: vec![Monitor::new("eDP-1".to_string(), 1, true)],
        });

        // 2nd update - change focused monitor (even if active workspace ID is same)
        let action = module.update(Event::HyprlandUpdate {
            workspaces: vec![Workspace::new(1, "eDP-1".to_string())],
            monitors: vec![Monitor::new("eDP-1".to_string(), 1, false)],
        });

        assert_eq!(action, UpdateAction::Redraw);
        assert_eq!(module.focused_monitor, ""); // none are focused in this mock result
    }
}
