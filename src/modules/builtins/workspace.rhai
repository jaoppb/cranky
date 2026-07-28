let active_bg = "#565f89";
let focused_bg = "#3b4261";
let border_radius = 0.0;

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
    if "border_radius" in config {
        border_radius = config.border_radius;
    }
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
                empty_ws.name = "special:" + state.special.to_string();
                empty_ws.monitor = m.name;
                workspaces.push(empty_ws);
            }
        }
    }
    
    // Sort workspaces by id after adding potentially missing active workspaces
    workspaces.sort(|a, b| if a.id < b.id { -1 } else if a.id > b.id { 1 } else { 0 });
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
    
    let inactive_color = "#7aa2f7";
    let active_text_color = "#ffffff";
    
    let children = [];
    
    for ws in workspaces {
        if ws.monitor == monitor_id {
            let label = ws.name;
            if label.starts_with("special:") {
                label = label.sub_string(8);
            }
            let current_active = if special_id != -1 && special_id != 0 { special_id } else { active_id };
            let is_visible = (ws.id == current_active);
            
            let color = inactive_color;
            if is_visible {
                color = active_text_color;
            }
            
            let on_click_cmd = #{};
            on_click_cmd["Exec"] = "hyprctl eval 'hl.dispatch(hl.dsp.focus({ workspace = " + ws.id.to_string() + " }))'";
            
            let text_node = #{
                type: "text",
                text: label,
                color: color,
                on_click: on_click_cmd
            };
            
            if is_visible {
                let bg = focused_bg;
                if is_monitor_focused {
                    bg = active_bg;
                }
                
                let box_node = #{
                    type: "flex",
                    children: [text_node],
                    background: bg,
                    radius: border_radius,
                    style: #{ padding: #{ top: 4.0, bottom: 4.0, left: 8.0, right: 8.0 }, justify: "center", align_items: "center" },
                    on_click: on_click_cmd
                };
                children.push(box_node);
            } else {
                let box_node = #{
                    type: "flex",
                    children: [text_node],
                    style: #{ padding: #{ top: 4.0, bottom: 4.0, left: 8.0, right: 8.0 }, justify: "center", align_items: "center" },
                    on_click: on_click_cmd
                };
                children.push(box_node);
            }
        }
    }
    
    let root = #{
        type: "flex",
        style: #{ gap: 6.0, align_items: "center" },
        children: children
    };
    
    root
}
