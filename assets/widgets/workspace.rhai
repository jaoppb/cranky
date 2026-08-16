let active_bg = "#3b4261";
let focused_bg = "#565f89";
let border_radius = 0.0;
let inactive_color = "#7aa2f7";
let active_text_color = "#ffffff";

let workspaces = [];
let active_workspaces = #{};
let focused_monitor = "";

fn init() {
    if "active" in config && "background_color" in config.active {
        active_bg = config.active.background_color;
    }
    if "focused" in config && "background_color" in config.focused {
        focused_bg = config.focused.background_color;
    }
    if "border_radius" in config { border_radius = config.border_radius; }
    if "inactive_color" in config { inactive_color = config.inactive_color; }
    if "active_text_color" in config { active_text_color = config.active_text_color; }
}

fn subscriptions() {
    ["hyprland"]
}

fn refresh() {
    if hyprland == () { return; }
    
    workspaces = hyprland.workspaces;
    if hyprland.focused_monitor != () {
        focused_monitor = hyprland.focused_monitor;
    } else {
        focused_monitor = "";
    }
    
    active_workspaces.clear();
    for m in hyprland.monitors {
        let state = #{};
        state.active = m.active_workspace_id;
        let t = type_of(m.special_workspace_id);
        if t == "i64" || t == "i32" {
            if m.special_workspace_id != 0 {
                state.special = m.special_workspace_id;
            }
        }
        active_workspaces[m.name] = state;
        
        let found = false;
        for ws in workspaces {
            if ws.id == m.active_workspace_id {
                found = true;
                break;
            }
        }
        if !found {
            let empty_ws = #{};
            empty_ws.id = m.active_workspace_id;
            empty_ws.name = m.active_workspace_id.to_string();
            empty_ws.monitor = m.name;
            workspaces.push(empty_ws);
        }

        if "special" in state {
            let found_sp = false;
            for ws in workspaces {
                if ws.id == state.special {
                    found_sp = true;
                    break;
                }
            }
            if !found_sp {
                let empty_ws = #{};
                empty_ws.id = state.special;
                empty_ws.name = "special";
                empty_ws.monitor = m.name;
                workspaces.push(empty_ws);
            }
        }
    }
    
    workspaces.sort(|a, b| {
        if a.id > 0 && b.id > 0 {
            if a.id < b.id { -1 } else if a.id > b.id { 1 } else { 0 }
        } else if a.id < 0 && b.id < 0 {
            if a.id < b.id { -1 } else if a.id > b.id { 1 } else { 0 }
        } else {
            if a.id > b.id { -1 } else { 1 }
        }
    });
}

fn render(monitor_id) {
    let active_id = -1;
    let special_id = -1;
    if monitor_id in active_workspaces {
        let state = active_workspaces[monitor_id];
        if "active" in state { active_id = state.active; }
        if "special" in state { special_id = state.special; }
    }
    
    let is_monitor_focused = (focused_monitor == monitor_id);
    
    let children = [];
    
    for ws in workspaces {
        if ws.monitor == monitor_id {
            let label = ws.name;
            let is_special = false;
            if label.starts_with("special:") {
                label = label.sub_string(8);
                is_special = true;
            } else if ws.id < 0 {
                is_special = true;
                if label == ws.id.to_string() { label = "special"; }
            }
            let current_active = if special_id != -1 && special_id != 0 { special_id } else { active_id };
            let is_visible = (ws.id == current_active);
            
            let color = inactive_color;
            if is_visible { color = active_text_color; }
            
            let on_click_cmd = #{};
            if is_special {
                on_click_cmd["Exec"] = "hyprctl dispatch togglespecialworkspace " + label;
            } else {
                on_click_cmd["Exec"] = "hyprctl dispatch workspace " + ws.id.to_string();
            }
            
            let text_node = #{
                type: "text",
                text: label,
                color: color
            };
            
            let tooltip_node = #{
                type: "flex",
                background: "#1e1e2eff",
                radius: 4.0,
                style: #{ padding: #{ top: 4.0, bottom: 4.0, left: 8.0, right: 8.0 } },
                children: [
                    #{ type: "text", text: "Switch to workspace " + label, color: "#ffffff" }
                ]
            };
            
            let box_node = #{
                type: "flex",
                children: [text_node],
                style: #{ padding: #{ top: 4.0, bottom: 4.0, left: 8.0, right: 8.0 }, justify: "center", align_items: "center" },
                on_click: on_click_cmd,
                tooltip: tooltip_node
            };
            
            if is_visible {
                let bg = active_bg;
                if is_monitor_focused {
                    bg = focused_bg;
                }
                box_node.background = bg;
                box_node.radius = border_radius;
            }
            
            children.push(box_node);
        }
    }
    
    #{
        type: "flex",
        style: #{ gap: 6.0, align_items: "center" },
        children: children
    }
}
