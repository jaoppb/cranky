let format_str = "%H:%M:%S";
let time_str = "";
let on_click_val = ();
let popup_monitors = #{};

fn init() {
    popup_monitors = #{};
    let options = if "options" in config { config.options } else if "module" in config { config["module"] } else { config };
    if "format" in options { format_str = options["format"]; }
    if "on_click" in options {
        on_click_val = ui.action.exec(options["on_click"]);
    } else {
        on_click_val = ui.action.script_call("toggle_popup");
    }
}

fn metadata() {
    #{
        subscriptions: ["time"],
        styles: ["clock"]
    }
}

fn get_monitor_key(mon) {
    if type_of(mon) == "string" {
        mon
    } else if type_of(mon) == "map" && "name" in mon {
        mon.name
    } else if type_of(mon) == "map" && "id" in mon {
        mon.id
    } else {
        "default"
    }
}

fn toggle_popup(mon) {
    let key = get_monitor_key(mon);
    if key in popup_monitors {
        popup_monitors.remove(key);
    } else {
        popup_monitors[key] = true;
    }
}

fn toggle_popup() {
    let key = "default";
    if key in popup_monitors {
        popup_monitors.remove(key);
    } else {
        popup_monitors[key] = true;
    }
}

fn on_popup_dismiss(mon) {
    if mon == () || mon == "default" {
        popup_monitors = #{};
    } else {
        let key = get_monitor_key(mon);
        if key in popup_monitors {
            popup_monitors.remove(key);
        }
    }
}

fn on_popup_dismiss() {
    popup_monitors = #{};
}

fn parse_dec(s) {
    let val = 0;
    let i = 0;
    while i < s.len() {
        let ch = s.sub_string(i, 1);
        if ch == "0" { val = val * 10 + 0; }
        else if ch == "1" { val = val * 10 + 1; }
        else if ch == "2" { val = val * 10 + 2; }
        else if ch == "3" { val = val * 10 + 3; }
        else if ch == "4" { val = val * 10 + 4; }
        else if ch == "5" { val = val * 10 + 5; }
        else if ch == "6" { val = val * 10 + 6; }
        else if ch == "7" { val = val * 10 + 7; }
        else if ch == "8" { val = val * 10 + 8; }
        else if ch == "9" { val = val * 10 + 9; }
        i = i + 1;
    }
    val
}

fn refresh() {
    if "time" in signals && signals.time != () {
        let t = signals.time.to_string();
        if t.len() >= 19 {
            let yr = t.sub_string(0, 4);
            let mo = t.sub_string(5, 2);
            let da = t.sub_string(8, 2);
            let h_24 = t.sub_string(11, 2);
            let m = t.sub_string(14, 2);
            let s = t.sub_string(17, 2);

            let h_val = parse_dec(h_24);
            let p = "AM";
            let h_12 = h_val;
            if h_val >= 12 { p = "PM"; }
            if h_12 > 12 { h_12 = h_12 - 12; }
            if h_12 == 0 { h_12 = 12; }
            let h_12_str = h_12.to_string();
            if h_12_str.len() == 1 { h_12_str = "0" + h_12_str; }

            let result = format_str;
            result.replace("%Y", yr);
            result.replace("%m", mo);
            result.replace("%d", da);
            result.replace("%H", h_24);
            result.replace("%M", m);
            result.replace("%S", s);
            result.replace("%I", h_12_str);
            result.replace("%p", p);
            result.replace("%R", `${h_24}:${m}`);
            result.replace("%T", `${h_24}:${m}:${s}`);

            time_str = result;
        } else {
            time_str = t;
        }
    }
}

fn render(monitor) {
    let node = #{
        class: "time",
        text: time_str
    };
    if on_click_val != () {
        node.on_click = on_click_val;
    }
    let key = get_monitor_key(monitor);
    if key in popup_monitors || "default" in popup_monitors {
        node.popup = ui.popup(#{
            // `module` is a reserved keyword in Rhai — `widget` is the alias.
            content: ui.widget("calendar", "calendar"),
            anchor: "bottom",
            dismiss_on_unfocus: true
        });
    }
    return ui.text(node);
}
