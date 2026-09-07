let format_str = "%H:%M:%S";
let time_str = "";
let on_click_val = ();
let popup_monitors = #{};
let first_day_of_week = "sunday";

let today_year = 0;
let today_month = 0;
let today_day = 0;
let view_year = 0;
let view_month = 0;

fn is_leap_year(y) {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

fn get_days_in_month(y, m) {
    if m == 2 && is_leap_year(y) {
        return 29;
    }
    let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    month_days[m - 1]
}

// Sakamoto's algorithm: 0 = Sunday, 1 = Monday, ..., 6 = Saturday
fn day_of_week(y, m, d) {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let adj_y = y;
    if m < 3 {
        adj_y = adj_y - 1;
    }
    (adj_y + adj_y / 4 - adj_y / 100 + adj_y / 400 + t[m - 1] + d) % 7
}

fn init() {
    popup_monitors = #{};
    let options = if "options" in config { config.options } else if "module" in config { config["module"] } else { config };
    if "format" in options { format_str = options["format"]; }
    if "first_day_of_week" in options { first_day_of_week = options["first_day_of_week"].to_lower(); }
    if "on_click" in options {
        on_click_val = ui.action.exec(options["on_click"]);
    } else {
        on_click_val = ui.action.script_call("toggle_popup");
    }
}

fn metadata() {
    #{
        subscriptions: ["time"],
        styles: ["clock", "calendar"]
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

fn prev_month() {
    view_month = view_month - 1;
    if view_month < 1 {
        view_month = 12;
        view_year = view_year - 1;
    }
}

fn next_month() {
    view_month = view_month + 1;
    if view_month > 12 {
        view_month = 1;
        view_year = view_year + 1;
    }
}

fn reset_today() {
    view_year = today_year;
    view_month = today_month;
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

            let ny = parse_dec(yr);
            let nm = parse_dec(mo);
            let nd = parse_dec(da);
            today_year = ny;
            today_month = nm;
            today_day = nd;
            if view_year == 0 {
                view_year = ny;
                view_month = nm;
            }

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

fn build_calendar_vnode(ui, v_year, v_month, t_year, t_month, t_day, first_dow_cfg) {
    let view_y = v_year;
    let view_m = v_month;
    if view_y == 0 {
        view_y = 2026;
        view_m = 1;
    }

    let month_names = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December"
    ];

    // Header
    let header_node = ui.flex(#{
        class: "header",
        children: [
            ui.text(#{
                class: "btn nav-btn",
                text: "<",
                on_click: ui.action.script_call("prev_month")
            }),
            ui.text(#{
                class: "title",
                text: `${month_names[view_m - 1]} ${view_y}`,
                on_click: ui.action.script_call("reset_today")
            }),
            ui.text(#{
                class: "btn nav-btn",
                text: ">",
                on_click: ui.action.script_call("next_month")
            })
        ]
    });

    // Weekdays
    let weekday_labels = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
    if first_dow_cfg == "monday" {
        weekday_labels = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
    }

    let weekday_children = [];
    for label in weekday_labels {
        weekday_children.push(ui.text(#{
            class: "weekday",
            text: label
        }));
    }

    let weekdays_node = ui.grid(#{
        class: "weekdays",
        children: weekday_children
    });

    // Days matrix
    let first_dow = day_of_week(view_y, view_m, 1);
    let leading_count = first_dow;
    if first_dow_cfg == "monday" {
        leading_count = (first_dow + 6) % 7;
    }

    let prev_m = view_m - 1;
    let prev_y = view_y;
    if view_m == 1 {
        prev_m = 12;
        prev_y = view_y - 1;
    }
    let prev_m_days = get_days_in_month(prev_y, prev_m);
    let cur_m_days = get_days_in_month(view_y, view_m);

    let day_children = [];

    // Leading days
    if leading_count > 0 {
        let i = leading_count - 1;
        while i >= 0 {
            let day_num = prev_m_days - i;
            day_children.push(ui.text(#{
                class: "day other-month",
                text: `${day_num}`
            }));
            i = i - 1;
        }
    }

    // Current month days
    let d = 1;
    while d <= cur_m_days {
        let is_today = (view_y == t_year && view_m == t_month && d == t_day);
        let class_name = "day";
        if is_today {
            class_name = "day today";
        }
        day_children.push(ui.text(#{
            class: class_name,
            text: `${d}`
        }));
        d = d + 1;
    }

    // Trailing days
    let trailing_count = 42 - day_children.len();
    let td = 1;
    while td <= trailing_count {
        day_children.push(ui.text(#{
            class: "day other-month",
            text: `${td}`
        }));
        td = td + 1;
    }

    let days_grid_node = ui.grid(#{
        class: "grid",
        children: day_children
    });

    ui.flex(#{
        class: "root",
        children: [
            header_node,
            weekdays_node,
            days_grid_node
        ]
    })
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
        let cal = build_calendar_vnode(ui, view_year, view_month, today_year, today_month, today_day, first_day_of_week);
        node.popup = ui.popup(#{
            content: cal,
            anchor: "bottom",
            dismiss_on_unfocus: true
        });
    }
    return ui.text(node);
}
