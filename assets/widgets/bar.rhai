let left = [];
let center = [];
let right = [];

fn init() {
    let options = if "options" in config { config.options } else if "module" in config { config["module"] } else { config };
    if "left" in options { left = options.left; }
    if "center" in options { center = options.center; }
    if "right" in options { right = options.right; }
}

fn metadata() {
    #{
        subscriptions: [],
        styles: ["bar"]
    }
}

fn refresh() {}

fn render(monitor) {
    let left_children = [];
    for mod_name in left {
        left_children.push(ui.widget(mod_name));
    }

    let center_children = [];
    for mod_name in center {
        center_children.push(ui.widget(mod_name));
    }

    let right_children = [];
    for mod_name in right {
        right_children.push(ui.widget(mod_name));
    }

    ui.flex(#{
        class: "root",
        children: [
            ui.flex(#{
                class: "left",
                children: left_children
            }),
            ui.flex(#{
                class: "center",
                children: center_children
            }),
            ui.flex(#{
                class: "right",
                children: right_children
            })
        ]
    })
}
