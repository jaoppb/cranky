let state = #{
    cpu: 0.0,
    ram_used: 0,
    ram_total: 0,
    net_tx: 0,
    net_rx: 0,
    temp: 0.0,
    disks: [],
    config: ()
};

fn init() {
}

fn metadata() {
    #{
        subscriptions: ["metrics"],
        styles: ["metrics"]
    }
}

fn refresh() {
    if metrics != () {
        state.cpu = metrics.cpu_usage;
        state.ram_used = metrics.memory_used;
        state.ram_total = metrics.memory_total;
        state.net_tx = metrics.network_tx;
        state.net_rx = metrics.network_rx;
        state.temp = metrics.temperature;
        state.disks = metrics.disks;
        state.config = metrics.config;
    }
}

fn format_bytes(bytes) {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let i = 0;
    let amount = bytes.to_float();
    while amount >= 1024.0 && i < (units.len() - 1) {
        amount = amount / 1024.0;
        i += 1;
    }
    let rounded = (amount * 10.0).to_int().to_float() / 10.0;
    rounded.to_string() + " " + units[i]
}

fn get_progress_class(percent) {
    if percent < 50.0 {
        return "progress";
    } else if percent < 80.0 {
        return "progress warning";
    } else {
        return "progress critical";
    }
}

fn is_enabled(metric_name, global_mode, cfg) {
    if metric_name in cfg && cfg[metric_name] == false {
        return false;
    }
    if global_mode == () || global_mode == "disabled" {
        return false;
    }
    true
}

fn get_widgets(s, cfg) {
    let widgets = [];
    if s.config == () { return widgets; }

    if "cpu" in s.config && is_enabled("cpu", s.config.cpu, cfg) {
        widgets.push(#{ type: "bar", label: "CPU", value: s.cpu, max: 100.0 });
    }

    if "memory" in s.config && is_enabled("memory", s.config.memory, cfg) {
        if s.config.memory == "absolute" {
            widgets.push(#{ type: "text", label: "RAM", text: format_bytes(s.ram_used) });
        } else {
            let percent = 0.0;
            if s.ram_total > 0 {
                percent = (s.ram_used.to_float() / s.ram_total.to_float()) * 100.0;
            }
            widgets.push(#{ type: "bar", label: "RAM", value: percent, max: 100.0 });
        }
    }

    if "disk" in s.config && is_enabled("disk", s.config.disk, cfg) && s.disks.len() > 0 {
        let d = s.disks[0];
        if s.config.disk == "absolute" {
            widgets.push(#{ type: "text", label: "DSK", text: format_bytes(d.used_bytes) });
        } else {
            let percent = 0.0;
            if d.total_bytes > 0 {
                percent = (d.used_bytes.to_float() / d.total_bytes.to_float()) * 100.0;
            }
            widgets.push(#{ type: "bar", label: "DSK", value: percent, max: 100.0 });
        }
    }

    if "network" in s.config && is_enabled("network", s.config.network, cfg) {
        let text_val = "▼" + format_bytes(s.net_rx) + " ▲" + format_bytes(s.net_tx);
        widgets.push(#{ type: "text", label: "NET", text: text_val });
    }

    if "temperature" in s.config && is_enabled("temperature", s.config.temperature, cfg) {
        let unit = if s.config.temperature == "celsius" { "°C" } else { "°F" };
        let rounded_temp = (s.temp * 10.0).to_int().to_float() / 10.0;
        let text_val = rounded_temp.to_string() + unit;
        widgets.push(#{ type: "text", label: "TMP", text: text_val });
    }

    widgets
}

fn render(monitor_id) {
    if state.config == () {
        return #{ type: "flex", class: "container", children: [] };
    }
    
    let widgets = get_widgets(state, config);
    if widgets.len() == 0 {
        return #{ type: "flex", class: "container", children: [] };
    }

    let children = [];

    for w in widgets {
        let widget_children = [];
        widget_children.push(#{
            type: "text",
            class: "label",
            text: w.label,
        });

        if w.type == "bar" {
            let percent = w.value;
            if percent > 100.0 { percent = 100.0; }
            if percent < 0.0 { percent = 0.0; }
            widget_children.push(#{
                type: "progress",
                class: get_progress_class(percent),
                value: percent / 100.0,
                orientation: "horizontal"
            });
        } else if w.type == "text" {
            widget_children.push(#{
                type: "text",
                class: "value",
                text: w.text,
            });
        }

        children.push(#{
            type: "flex",
            class: "item",
            children: widget_children
        });
    }

    #{
        type: "flex",
        class: "container",
        children: children
    }
}
