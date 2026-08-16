let show_titles = true;
let show_icons = true;
let icon_size = 16;
let max_items = 6;
let empty_label = "applet: none";

let items = [];

fn init() {
    if "show_titles" in config { show_titles = config["show_titles"]; }
    if "show_icons" in config { show_icons = config["show_icons"]; }
    if "icon_size" in config { icon_size = config["icon_size"].to_int(); }
    if "max_items" in config { max_items = config["max_items"].to_int(); }
    if "empty_label" in config { empty_label = config["empty_label"]; }
}

fn subscriptions() {
    ["applets"]
}

fn refresh() {
    if applets != () {
        items = applets;
    } else {
        items = [];
    }
}

fn render(monitor_id) {
    let text_color = "#c0caf5";
    
    if items.len() == 0 {
        return #{
            type: "text",
            text: empty_label,
            color: text_color,
            tooltip: #{
                type: "flex",
                background: "#1e1e2eff",
                radius: 4.0,
                style: #{ padding: #{ top: 4.0, bottom: 4.0, left: 8.0, right: 8.0 } },
                children: [
                    #{ type: "text", text: "No applets are currently active", color: text_color }
                ]
            }
        };
    }

    let children = [];
    let i = 0;
    for item in items {
        i += 1;
        if i > max_items { break; }
        
        let item_children = [];
        
        if show_icons {
            let img = ();
            if "icon" in item && item.icon != () {
                let icon_val = item.icon;
                if type_of(icon_val) == "map" && "image" in icon_val {
                    img = icon_val.image;
                }
            }
            if img != () && type_of(img) == "map" {
                item_children.push(#{
                    type: "image",
                    size: #{ width: icon_size, height: icon_size },
                    data: img.data,
                    pixel_size: img.size
                });
            } else {
                item_children.push(#{
                    type: "rect",
                    size: #{ width: icon_size, height: icon_size },
                    color: text_color
                });
            }
        }
        
        if show_titles {
            let label = "app";
            if "title" in item && item.title != () && item.title != "" {
                label = item.title;
            } else if "item_id" in item && item.item_id != () && item.item_id != "" {
                label = item.item_id;
            }
            item_children.push(#{
                type: "text",
                text: label,
                color: text_color
            });
        }
        
        let tooltip_children = [];
        if "tooltip" in item && item.tooltip != () && type_of(item.tooltip) == "map" {
            let t = "";
            let d = "";
            if "title" in item.tooltip && item.tooltip.title != () {
                t = item.tooltip.title;
            }
            if "description" in item.tooltip && item.tooltip.description != () {
                d = item.tooltip.description;
            }
            if t != "" {
                tooltip_children.push(#{ type: "text", text: t, color: text_color });
            }
            if d != "" && d != t {
                tooltip_children.push(#{ type: "text", text: d, color: text_color });
            }
        }
        if tooltip_children.len() == 0 {
            let fallback = "app";
            if "title" in item && item.title != () && item.title != "" {
                fallback = item.title;
            } else if "item_id" in item && item.item_id != () && item.item_id != "" {
                fallback = item.item_id;
            }
            tooltip_children.push(#{ type: "text", text: fallback, color: text_color });
        }
        
        let applet_node = #{
            type: "flex",
            style: #{ gap: 6.0, align_items: "center" },
            children: item_children,
            on_click: #{
                AppletAction: #{
                    id: item.id,
                    action: "Primary"
                }
            },
            tooltip: #{
                type: "flex",
                style: #{ padding: #{ top: 6.0, bottom: 6.0, left: 10.0, right: 10.0 }, gap: 4.0, direction: "column" },
                background: "#1e1e2eff",
                radius: 6.0,
                children: tooltip_children
            }
        };
        children.push(applet_node);
    }
    
    #{
        type: "flex",
        style: #{ gap: 8.0, align_items: "center" },
        children: children
    }
}
