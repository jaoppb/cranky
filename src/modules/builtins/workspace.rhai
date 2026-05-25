let active_bg = "#565f89";
let focused_bg = "#3b4261";
let border_radius = 0.0;
let font_family = "";
let font_size = 14.0;

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
    font_family = bar_config.font_family;
    font_size = bar_config.font_size;
}

fn subscriptions() {
    ["hyprland"]
}

fn refresh() {
    if hyprland == () { return; }
    
    workspaces = hyprland.workspaces;
    
    // Sort workspaces by id using a simple bubble sort since Rhai's sort requires a function pointer which is tricky
    // but Rhai arrays have a .sort() method if we provide a simple comparison or if elements are comparable!
    // We can extract a list of objects and sort them.
    workspaces.sort(|a, b| if a.id < b.id { -1 } else if a.id > b.id { 1 } else { 0 });
    
    active_workspaces.clear();
    for m in hyprland.monitors {
        active_workspaces[m.name] = m.active_workspace_id;
        if m.focused {
            focused_monitor = m.name;
        }
    }
}

fn measure(monitor_id) {
    let count = 0;
    for ws in workspaces {
        if ws.monitor == monitor_id {
            count += 1;
        }
    }
    
    if count == 0 { return [0, 0]; }
    
    let text_size = measure_text("0", font_family, font_size);
    let padding_y = 4.0;
    let item_size = text_size[1].to_float() + padding_y * 2.0;
    let item_spacing = 6.0;
    
    let c = count;
    if c < 1 { c = 1; }
    
    let width = (c - 1).to_float() * item_spacing + item_size * count.to_float();
    [width.to_int(), item_size.to_int()]
}

fn view(monitor_id) {
    let active_id = -1;
    if monitor_id in active_workspaces {
        active_id = active_workspaces[monitor_id];
    }
    
    let is_monitor_focused = (focused_monitor == monitor_id);
    
    let text_size = measure_text("0", font_family, font_size);
    let padding_y = 4.0;
    let padding_x = 6.0;
    let item_size = text_size[1].to_float() + padding_y * 2.0;
    let item_spacing = 6.0;
    let x_offset = 0.0;
    
    let inactive_color = "#7aa2f7";
    let active_text_color = "#ffffff";
    
    for ws in workspaces {
        if ws.monitor == monitor_id {
            let label = ws.id.to_string();
            let is_visible = (ws.id == active_id);
            let ws_text_size = measure_text(label, font_family, font_size);
            let lw = ws_text_size[0].to_float();
            
            if is_visible {
                let bg = focused_bg;
                if is_monitor_focused {
                    bg = active_bg;
                }
                let rect_w = lw + padding_x * 2.0;
                if rect_w < item_size { rect_w = item_size; }
                draw_rect(x_offset, 0.0, rect_w, item_size, bg, border_radius);
                
                draw_text(label, font_family, font_size, active_text_color, x_offset + (rect_w - lw) / 2.0, padding_y);
                x_offset += rect_w + item_spacing;
            } else {
                let rect_w = lw + padding_x * 2.0;
                if rect_w < item_size { rect_w = item_size; }
                draw_text(label, font_family, font_size, inactive_color, x_offset + (rect_w - lw) / 2.0, padding_y);
                x_offset += rect_w + item_spacing;
            }
        }
    }
}

fn on_event(event) {
    if event.type == "click" && event.button == 272 {
        let text_size = measure_text("0", font_family, font_size);
        let lh = text_size[1].to_float();
        let padding_x = 6.0;
        let rect_w = 16.0 + padding_x * 2.0;
        if rect_w < lh + 8.0 { rect_w = lh + 8.0; }
        let item_spacing = 6.0;
        let index = (event.x / (rect_w + item_spacing)).to_int() + 1;
        
        let current = 1;
        for ws in workspaces {
            if current == index {
                exec("hyprctl dispatch workspace " + ws.id.to_string());
                break;
            }
            current += 1;
        }
    }
}
