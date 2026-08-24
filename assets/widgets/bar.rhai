let left = [];
let center = [];
let right = [];

fn init() {
    if "left" in config { left = config["left"]; }
    if "center" in config { center = config["center"]; }
    if "right" in config { right = config["right"]; }
}

fn metadata() {
    #{
        subscriptions: [],
        styles: ["bar"]
    }
}

fn refresh() {
}

fn render(monitor_id) {
    let left_children = [];
    for mod_name in left {
        left_children.push(#{
            type: "module",
            name: mod_name
        });
    }

    let center_children = [];
    for mod_name in center {
        center_children.push(#{
            type: "module",
            name: mod_name
        });
    }

    let right_children = [];
    for mod_name in right {
        right_children.push(#{
            type: "module",
            name: mod_name
        });
    }

    #{
        type: "flex",
        class: "root",
        children: [
            #{
                type: "flex",
                class: "left",
                children: left_children
            },
            #{
                type: "flex",
                class: "center",
                children: center_children
            },
            #{
                type: "flex",
                class: "right",
                children: right_children
            }
        ]
    }
}
