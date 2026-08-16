let player_map = #{};
let player_list = [];
let active_idx = 0;
let text_color = "#c0caf5";
let accent_color = "#7aa2f7";

fn init() {
    if "text_color" in config { text_color = config.text_color; }
    if "accent_color" in config { accent_color = config.accent_color; }
}

fn subscriptions() {
    return ["mpris"];
}

fn refresh() {
    if "players" in mpris {
        player_map = mpris.players;
        player_list = player_map.keys();
        player_list.sort();

        // Clamp active_idx to valid range
        if active_idx >= player_list.len() { active_idx = player_list.len() - 1; }
        if active_idx < 0 { active_idx = 0; }
    } else {
        player_map = #{};
        player_list = [];
        active_idx = 0;
    }
}

// ScriptCall callbacks for player cycling
fn cycle_prev() {
    if player_list.len() <= 1 { return; }
    active_idx = active_idx - 1;
    if active_idx < 0 { active_idx = player_list.len() - 1; }
}

fn cycle_next() {
    if player_list.len() <= 1 { return; }
    active_idx = active_idx + 1;
    if active_idx >= player_list.len() { active_idx = 0; }
}

fn get_active_player() {
    if player_list.len() == 0 { return [(), ""]; }
    let bus_name = player_list[active_idx];
    return [player_map[bus_name], bus_name];
}

fn get_player_display_name(idx) {
    let bus = player_list[idx];
    let p = player_map[bus];
    if "name" in p {
        return p.name;
    }
    return bus;
}

fn prev_player_name() {
    if player_list.len() <= 1 { return (); }
    let idx = active_idx - 1;
    if idx < 0 { idx = player_list.len() - 1; }
    return get_player_display_name(idx);
}

fn next_player_name() {
    if player_list.len() <= 1 { return (); }
    let idx = active_idx + 1;
    if idx >= player_list.len() { idx = 0; }
    return get_player_display_name(idx);
}

fn create_button(icon, click_action, tooltip_text, color) {
    let ttip = ();
    if tooltip_text != () {
        ttip = #{
            type: "flex",
            background: "#1e1e2eff",
            radius: 4,
            style: #{ padding: #{ top: 4.0, bottom: 4.0, left: 8.0, right: 8.0 } },
            children: [
                #{ type: "text", text: tooltip_text, color: "#ffffff" }
            ]
        };
    }

    return #{
        type: "flex",
        style: #{ 
            justify: "center", 
            align_items: "center", 
            padding: #{ top: 4.0, bottom: 4.0, left: 8.0, right: 8.0 } 
        },
        background: "#313244",
        radius: 4.0,
        on_click: click_action,
        tooltip: ttip,
        children: [
            #{ type: "text", text: icon, color: color }
        ]
    };
}

fn render(monitor) {
    let p_res = get_active_player();
    if p_res == () {
        return #{ type: "flex", children: [] };
    }
    
    let p = p_res[0];
    let bus_name = p_res[1];

    let title = "Unknown";
    if "track_name" in p {
        if p.track_name != () {
            title = p.track_name;
        }
    }

    let artist = "";
    if "artist" in p {
        if p.artist != () {
            artist = p.artist;
        }
    }

    let is_playing = (p.status == "playing");

    let display_text = title;
    if artist != "" {
        display_text = artist + " — " + title;
    }

    let children = [];

    // Player cycling: left arrow
    if player_list.len() > 1 {
        let prev_name = prev_player_name();
        children.push(create_button("󰅁", #{ ScriptCall: "cycle_prev" }, prev_name, text_color));
    }

    // Prev Track button
    children.push(create_button("󰒮", #{ Exec: "playerctl --player=" + bus_name + " previous" }, "Previous Track", text_color));

    // Play/Pause button
    let status_icon = "󰐊";
    if is_playing { status_icon = "󰏤"; }
    let play_pause_ttip = "Play";
    if is_playing { play_pause_ttip = "Pause"; }
    children.push(create_button(status_icon, #{ Exec: "playerctl --player=" + bus_name + " play-pause" }, play_pause_ttip, accent_color));

    // Next Track button
    children.push(create_button("󰒭", #{ Exec: "playerctl --player=" + bus_name + " next" }, "Next Track", text_color));

    // Player cycling: right arrow
    if player_list.len() > 1 {
        let nxt = next_player_name();
        children.push(create_button("󰅂", #{ ScriptCall: "cycle_next" }, nxt, text_color));
    }

    let max_width = 30;
    let truncated_text = display_text;
    if display_text.len() > max_width {
        truncated_text = display_text.sub_string(0, max_width - 3) + "...";
    }

    let artist_disp = "Unknown";
    if artist != "" { artist_disp = artist; }
    
    let player_disp = bus_name;
    if "name" in p {
        if p.name != () { player_disp = p.name; }
    }

    // Track info
    children.push(#{
        type: "flex",
        style: #{ justify: "center", align_items: "center", padding: #{ top: 4.0, bottom: 4.0, left: 8.0, right: 8.0 } },
        background: "#1e1e2e",
        radius: 4.0,
        tooltip: #{
            type: "flex",
            background: "#1e1e2eff",
            radius: 4,
            style: #{ padding: #{ top: 8.0, bottom: 8.0, left: 12.0, right: 12.0 }, gap: 4.0, direction: "column" },
            children: [
                #{ type: "text", text: "Title: " + title, color: "#ffffff" },
                #{ type: "text", text: "Artist: " + artist_disp, color: text_color },
                #{ type: "text", text: "Status: " + p.status, color: text_color },
                #{ type: "text", text: "Player: " + player_disp, color: accent_color }
            ]
        },
        children: [
            #{ type: "text", text: truncated_text, color: text_color }
        ]
    });

    return #{
        type: "flex",
        style: #{ gap: 6.0, align_items: "center" },
        children: children,
    };
}
