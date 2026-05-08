use crate::core::hyprland::Workspace;
use crate::modules::CrankyModule;
use crate::ports::canvas::Canvas;
use crate::domain::signals::SignalHub;
use crate::domain::errors::DomainError;
use crate::domain::color::DrawingColor;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct WorkspaceConfig {
    #[serde(default)]
    active: ActiveWorkspaceConfig,
    #[serde(default)]
    focused: FocusedWorkspaceConfig,
    #[serde(default)]
    border_radius: f32,
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
    background_color: DrawingColor,
}

impl Default for ActiveWorkspaceConfig {
    fn default() -> Self {
        Self {
            background_color: default_active_bg(),
        }
    }
}

fn default_active_bg() -> DrawingColor {
    DrawingColor::parse("#565f89").unwrap()
}

#[derive(Debug, Deserialize, Clone)]
pub struct FocusedWorkspaceConfig {
    #[serde(default = "default_focused_bg")]
    background_color: DrawingColor,
}

impl Default for FocusedWorkspaceConfig {
    fn default() -> Self {
        Self {
            background_color: default_focused_bg(),
        }
    }
}

fn default_focused_bg() -> DrawingColor {
    DrawingColor::parse("#3b4261").unwrap()
}

pub struct WorkspaceModule {
    workspaces: Vec<Workspace>,
    active_workspaces: HashMap<String, i32>,
    focused_monitor: String,
    active_background: DrawingColor,
    focused_background: DrawingColor,
    border_radius: f32,
}

impl WorkspaceModule {
    pub fn new() -> Self {
        Self {
            workspaces: Vec::new(),
            active_workspaces: HashMap::new(),
            focused_monitor: String::new(),
            active_background: default_active_bg(),
            focused_background: default_focused_bg(),
            border_radius: 0.0,
        }
    }
}

impl<C: Canvas> CrankyModule<C> for WorkspaceModule {
    type Config = WorkspaceConfig;

    fn init(
        &mut self,
        config: Self::Config,
        _bar_config: &crate::config::BarConfig,
    ) -> Result<(), DomainError> {
        self.active_background = config.active.background_color.clone();
        self.focused_background = config.focused.background_color.clone();
        self.border_radius = config.border_radius;
        Ok(())
    }

    fn attach(&mut self, hub: &SignalHub, target_id: u32) {
        let mut hypr_rx = hub.hyprland_rx();
        let dirty_tx = hub.dirty_tx();
        
        tokio::spawn(async move {
            while hypr_rx.changed().await.is_ok() {
                let _ = dirty_tx.send(target_id).await;
            }
        });
    }

    fn refresh(&mut self, hub: &SignalHub) {
        let state = hub.hyprland_rx().borrow().clone();
        
        let mut new_workspaces = state.workspaces().to_vec();
        new_workspaces.sort_by_key(|w| w.id());
        self.workspaces = new_workspaces;

        for m in state.monitors() {
            self.active_workspaces
                .insert(m.name().to_string(), m.active_workspace_id());
            
            if m.focused() {
                self.focused_monitor = m.name().to_string();
            }
        }
    }

    fn view(&self, canvas: &mut C, monitor: &str) {
        let monitor_workspaces: Vec<&Workspace> = self
            .workspaces
            .iter()
            .filter(|w| w.monitor() == monitor)
            .collect();

        let active_id = self.active_workspaces.get(monitor).cloned().unwrap_or(-1);
        let is_monitor_focused = self.focused_monitor == monitor;

        let item_size = 24.0;
        let item_spacing = 30.0;
        let mut x_offset = 0.0;

        let inactive_color = DrawingColor::parse("#7aa2f7").unwrap();
        let active_text_color = DrawingColor::parse("#ffffff").unwrap();

        for ws in monitor_workspaces {
            let label = ws.id().to_string();
            let is_visible = ws.id() == active_id;

            if is_visible {
                let background_color = if is_monitor_focused {
                    &self.active_background
                } else {
                    &self.focused_background
                };

                canvas.draw_rect(
                    x_offset,
                    (30.0 - item_size) / 2.0, // Hardcoded bar height for now, Phase 3 will fix
                    item_size,
                    item_size,
                    background_color.clone(),
                    self.border_radius,
                );

                let (label_width, _) = canvas.measure_text(&label, "", 14.0);
                canvas.draw_text(
                    &label,
                    "",
                    14.0,
                    active_text_color.clone(),
                    x_offset + (item_size - label_width) / 2.0,
                    15.0,
                );
            } else {
                let (label_width, _) = canvas.measure_text(&label, "", 14.0);
                canvas.draw_text(
                    &label,
                    "",
                    14.0,
                    inactive_color.clone(),
                    x_offset + (item_size - label_width) / 2.0,
                    15.0,
                );
            }
            x_offset += item_spacing;
        }
    }

    fn measure(&self, _canvas: &mut C, monitor: &str) -> (f32, f32) {
        let count = self.workspaces.iter().filter(|w| w.monitor() == monitor).count();
        (count as f32 * 30.0, 30.0)
    }
}
