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
    if "first_day_of_week" in config {
        first_day_of_week = config["first_day_of_week"].to_lower();
    }
}

fn metadata() {
    #{
        subscriptions: ["time"],
        styles: ["calendar"]
    }
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
    if current_time != () {
        let t = current_time.to_string();
        if t.len() >= 10 {
            let y_str = t.sub_string(0, 4);
            let m_str = t.sub_string(5, 2);
            let d_str = t.sub_string(8, 2);
            let ny = parse_dec(y_str);
            let nm = parse_dec(m_str);
            let nd = parse_dec(d_str);
            today_year = ny;
            today_month = nm;
            today_day = nd;
            if view_year == 0 {
                view_year = ny;
                view_month = nm;
            }
        }
    }
}

fn render(monitor_id) {
    if view_year == 0 {
        view_year = 2026;
        view_month = 1;
    }

    let month_names = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December"
    ];

    // Header
    let header_node = #{
        type: "flex",
        class: "header",
        children: [
            #{
                type: "text",
                class: "btn nav-btn",
                text: "<",
                on_click: #{ ScriptCall: "prev_month" }
            },
            #{
                type: "text",
                class: "title",
                text: `${month_names[view_month - 1]} ${view_year}`,
                on_click: #{ ScriptCall: "reset_today" }
            },
            #{
                type: "text",
                class: "btn nav-btn",
                text: ">",
                on_click: #{ ScriptCall: "next_month" }
            }
        ]
    };

    // Weekdays
    let weekday_labels = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
    if first_day_of_week == "monday" {
        weekday_labels = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
    }

    let weekday_children = [];
    for label in weekday_labels {
        weekday_children.push(#{
            type: "text",
            class: "weekday",
            text: label
        });
    }

    let weekdays_node = #{
        type: "grid",
        class: "weekdays",
        children: weekday_children
    };

    // Days matrix
    let first_dow = day_of_week(view_year, view_month, 1);
    let leading_count = first_dow;
    if first_day_of_week == "monday" {
        leading_count = (first_dow + 6) % 7;
    }

    let prev_m = view_month - 1;
    let prev_y = view_year;
    if view_month == 1 {
        prev_m = 12;
        prev_y = view_year - 1;
    }
    let prev_m_days = get_days_in_month(prev_y, prev_m);
    let cur_m_days = get_days_in_month(view_year, view_month);

    let day_children = [];

    // Leading days
    if leading_count > 0 {
        let i = leading_count - 1;
        while i >= 0 {
            let day_num = prev_m_days - i;
            day_children.push(#{
                type: "text",
                class: "day other-month",
                text: `${day_num}`
            });
            i = i - 1;
        }
    }

    // Current month days
    let d = 1;
    while d <= cur_m_days {
        let is_today = (view_year == today_year && view_month == today_month && d == today_day);
        let class_name = "day";
        if is_today {
            class_name = "day today";
        }
        day_children.push(#{
            type: "text",
            class: class_name,
            text: `${d}`
        });
        d = d + 1;
    }

    // Trailing days
    let trailing_count = 42 - day_children.len();
    let td = 1;
    while td <= trailing_count {
        day_children.push(#{
            type: "text",
            class: "day other-month",
            text: `${td}`
        });
        td = td + 1;
    }

    let days_grid_node = #{
        type: "grid",
        class: "grid",
        children: day_children
    };

    return #{
        type: "flex",
        class: "root",
        children: [
            header_node,
            weekdays_node,
            days_grid_node
        ]
    };
}
