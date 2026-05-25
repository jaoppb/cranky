let format = "%H:%M:%S";
let time_str = "";

fn init() {
    if "format" in config {
        format = config.format;
    }
}

fn subscriptions() {
    ["time"]
}

fn refresh() {
    // current_time is passed from Rust as RFC3339
    // Rhai does not have a built-in formatting for time out of the box,
    // so we just return the time_str string if we don't have it,
    // but typically we can do simple substring or replace.
    // For now, we'll parse the HH:MM:SS from RFC3339 manually
    // e.g., "2023-10-15T14:30:00+00:00"
    if current_time != () {
        let t = current_time.to_string();
        if t.len() >= 19 {
            if format == "%H:%M:%S" {
                time_str = t.sub_string(11, 8); // Extracts HH:MM:SS
            } else {
                let h = t.sub_string(11, 2).parse_int();
                let m = t.sub_string(14, 2);
                let s = t.sub_string(17, 2);
                let p = "AM";
                if h >= 12 { p = "PM"; }
                if h > 12 { h = h - 12; }
                if h == 0 { h = 12; }
                let h_str = h.to_string();
                if h_str.len() == 1 { h_str = "0" + h_str; }
                time_str = h_str + ":" + m + ":" + s + " " + p;
            }
        } else {
            time_str = t;
        }
    }
}

fn measure(monitor) {
    measure_text(time_str, bar_config.font_family, bar_config.font_size)
}

fn view(monitor) {
    let color = "#c0caf5";
    draw_text(time_str, bar_config.font_family, bar_config.font_size, color, 0.0, 0.0);
}

fn on_event(event) {
    if event.type == "click" && event.button == 272 {
        if format == "%H:%M:%S" {
            format = "%I:%M:%S %p";
        } else {
            format = "%H:%M:%S";
        }
        refresh();
    }
}
