let player_map = #{};
let player_list = [];
let active_idx = 0;

fn init() {
}

fn metadata() {
    #{
        subscriptions: ["mpris"],
        styles: ["mpris"]
    }
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

fn create_button(icon, click_action, tooltip_text, class_name) {
    let ttip = ();
    if tooltip_text != () {
        ttip = #{
            type: "flex",
            class: "tooltip",
            children: [
                #{ type: "text", text: tooltip_text }
            ]
        };
    }

    return #{
        type: "flex",
        class: class_name,
        on_click: click_action,
        tooltip: ttip,
        children: [
            #{ type: "text", text: icon }
        ]
    };
}

fn render(monitor) {
    let p_res = get_active_player();
    if p_res == () {
        return #{ type: "flex", class: "container", children: [] };
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
        children.push(create_button("󰅁", #{ ScriptCall: "cycle_prev" }, prev_name, "btn-cycle"));
    }

    // Prev Track button
    children.push(create_button("󰒮", #{ Exec: "playerctl --player=" + bus_name + " previous" }, "Previous Track", "btn-prev"));

    // Play/Pause button
    let status_icon = "󰐊";
    if is_playing { status_icon = "󰏤"; }
    let play_pause_ttip = "Play";
    if is_playing { play_pause_ttip = "Pause"; }
    children.push(create_button(status_icon, #{ Exec: "playerctl --player=" + bus_name + " play-pause" }, play_pause_ttip, "btn-play"));

    // Next Track button
    children.push(create_button("󰒭", #{ Exec: "playerctl --player=" + bus_name + " next" }, "Next Track", "btn-next"));

    // Player cycling: right arrow
    if player_list.len() > 1 {
        let nxt = next_player_name();
        children.push(create_button("󰅂", #{ ScriptCall: "cycle_next" }, nxt, "btn-cycle"));
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
        class: "track-info",
        tooltip: #{
            type: "flex",
            class: "tooltip",
            children: [
                #{ type: "text", class: "title", text: "Title: " + title },
                #{ type: "text", class: "artist", text: "Artist: " + artist_disp },
                #{ type: "text", class: "status", text: "Status: " + p.status },
                #{ type: "text", class: "player", text: "Player: " + player_disp }
            ]
        },
        children: [
            #{ type: "text", text: truncated_text }
        ]
    });

    return #{
        type: "flex",
        class: "container",
        children: children,
    };
}
