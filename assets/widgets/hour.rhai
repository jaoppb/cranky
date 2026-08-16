let format_str = "%H:%M:%S";
let time_str = "";
let text_color = "#c0caf5";
let font_val = ();
let size_val = ();
let tooltip_val = ();
let on_click_val = ();

fn init() {
    if "format" in config { format_str = config["format"]; }
    if "color" in config { text_color = config["color"]; }
    if "font" in config { font_val = config["font"]; }
    if "size" in config { size_val = config["size"]; }
    if "tooltip" in config { tooltip_val = config["tooltip"]; }
    if "on_click" in config { on_click_val = #{ Exec: config["on_click"] }; }
}

fn subscriptions() {
    ["time"]
}

fn refresh() {
    if current_time != () {
        let t = current_time.to_string();
        if t.len() >= 19 {
            let yr = t.sub_string(0, 4);
            let mo = t.sub_string(5, 2);
            let da = t.sub_string(8, 2);
            let h_24 = t.sub_string(11, 2);
            let m = t.sub_string(14, 2);
            let s = t.sub_string(17, 2);

            let h_val = h_24.parse_int();
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

fn render(monitor_id) {
    let node = #{
        type: "text",
        text: time_str,
        color: text_color
    };
    if font_val != () { node.font = font_val; }
    if size_val != () { node.size = size_val; }
    if tooltip_val != () { node.tooltip = tooltip_val; }
    if on_click_val != () { node.on_click = on_click_val; }
    node
}
