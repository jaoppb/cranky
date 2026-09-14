let show_titles = true;
let show_icons = true;
let max_items = 6;
let empty_label = "systray: none";

let items = [];

fn init() {
    let options = if "options" in config { config.options } else if "module" in config { config["module"] } else { config };
    if "show_titles" in options { show_titles = options["show_titles"]; }
    if "show_icons" in options { show_icons = options["show_icons"]; }
    if "max_items" in options { max_items = options["max_items"].to_int(); }
    if "empty_label" in options { empty_label = options["empty_label"]; }
}

fn metadata() {
    #{
        subscriptions: ["systray"],
        styles: ["systray"]
    }
}

fn refresh() {
    if "systray" in signals && signals.systray != () {
        items = signals.systray;
    } else {
        items = [];
    }
}

fn render(monitor) {
    if items.len() == 0 {
        return ui.text(#{
            class: "empty",
            text: empty_label,
            tooltip: ui.flex(#{
                class: "tooltip",
                children: [
                    ui.text(#{ text: "No systray items are currently active" })
                ]
            })
        });
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
                item_children.push(ui.image(#{
                    class: "icon",
                    data: img.data,
                    pixel_size: img.size
                }));
            } else {
                item_children.push(ui.rect(#{
                    class: "icon-placeholder"
                }));
            }
        }
        
        if show_titles {
            let label = "app";
            if "title" in item && item.title != () && item.title != "" {
                label = item.title;
            } else if "item_id" in item && item.item_id != () && item.item_id != "" {
                label = item.item_id;
            }
            item_children.push(ui.text(#{
                class: "title",
                text: label
            }));
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
                tooltip_children.push(ui.text(#{ class: "tooltip-title", text: t }));
            }
            if d != "" && d != t {
                tooltip_children.push(ui.text(#{ class: "tooltip-desc", text: d }));
            }
        }
        if tooltip_children.len() == 0 {
            let fallback = "app";
            if "title" in item && item.title != () && item.title != "" {
                fallback = item.title;
            } else if "item_id" in item && item.item_id != () && item.item_id != "" {
                fallback = item.item_id;
            }
            tooltip_children.push(ui.text(#{ class: "tooltip-title", text: fallback }));
        }
        
        let item_node = ui.flex(#{
            class: "item",
            key: item.id.to_string(),
            children: item_children,
            on_click: #{
                left: ui.action.systray(item.id, "Primary"),
                right: ui.action.systray(item.id, "ContextMenu")
            },
            tooltip: ui.flex(#{
                class: "tooltip",
                children: tooltip_children
            })
        });
        
        children.push(item_node);
    }
    
    ui.flex(#{
        class: "container",
        children: children
    })
}
