let workspaces = [];
let active_workspaces = #{};
let focused_monitor = "";

fn init() {
}

fn metadata() {
    #{
        subscriptions: ["hyprland"],
        styles: ["workspace"]
    }
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
            
            let on_click_cmd = #{};
            if is_special {
                on_click_cmd["Exec"] = "hyprctl dispatch 'hl.dsp.workspace.toggle_special(\"" + label + "\")'";
            } else {
                on_click_cmd["Exec"] = "hyprctl dispatch 'hl.dsp.focus({ workspace = \"" + ws.id.to_string() + "\"})'";
            }
            
            let text_node = #{
                type: "text",
                class: "label",
                text: label
            };
            
            let tooltip_node = #{
                type: "flex",
                class: "tooltip",
                children: [
                    #{ type: "text", text: "Switch to workspace " + label }
                ]
            };
            
            let classes = "item";
            if is_visible {
                if is_monitor_focused {
                    classes = "item active focused";
                } else {
                    classes = "item active";
                }
            }

            let box_node = #{
                type: "flex",
                class: classes,
                id: "ws-" + ws.id.to_string(),
                children: [text_node],
                on_click: on_click_cmd,
                tooltip: tooltip_node
            };
            
            children.push(box_node);
        }
    }
    
    #{
        type: "flex",
        class: "container",
        children: children
    }
}
