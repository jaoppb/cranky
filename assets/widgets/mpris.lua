local player_map = {}
local player_list = {}
local active_idx = 1
local text_color = "#c0caf5"
local accent_color = "#7aa2f7"

function init()
    if config.text_color then text_color = config.text_color end
    if config.accent_color then accent_color = config.accent_color end
end

function subscriptions()
    return { "mpris" }
end

function refresh()
    if mpris and mpris.players then
        player_map = mpris.players
        player_list = {}
        for name, _ in pairs(player_map) do
            table.insert(player_list, name)
        end
        table.sort(player_list)

        -- Clamp active_idx to valid range
        if active_idx > #player_list then active_idx = #player_list end
        if active_idx < 1 then active_idx = 1 end
    else
        player_map = {}
        player_list = {}
        active_idx = 1
    end
end

-- ScriptCall callbacks for player cycling
function cycle_prev()
    if #player_list <= 1 then return end
    active_idx = active_idx - 1
    if active_idx < 1 then active_idx = #player_list end
end

function cycle_next()
    if #player_list <= 1 then return end
    active_idx = active_idx + 1
    if active_idx > #player_list then active_idx = 1 end
end

local function get_active_player()
    if #player_list == 0 then return nil end
    local bus_name = player_list[active_idx]
    return player_map[bus_name], bus_name
end

local function get_player_display_name(idx)
    local bus = player_list[idx]
    local p = player_map[bus]
    -- Since we didn't implement identity VO properly in the backend yet (we used PlayerName as string)
    -- we just return the name field.
    return (p and p.name) or bus
end

local function prev_player_name()
    if #player_list <= 1 then return nil end
    local idx = active_idx - 1
    if idx < 1 then idx = #player_list end
    return get_player_display_name(idx)
end

local function next_player_name()
    if #player_list <= 1 then return nil end
    local idx = active_idx + 1
    if idx > #player_list then idx = 1 end
    return get_player_display_name(idx)
end

local function create_button(icon, cmd, tooltip_text, color)
    local ttip = nil
    if tooltip_text then
        ttip = {
            type = "flex",
            background = "#1e1e2eff",
            radius = 4,
            style = { padding = { top = 4, bottom = 4, left = 8, right = 8 } },
            children = {
                { type = "text", text = tooltip_text, color = "#ffffff" }
            }
        }
    end

    local click_action
    if type(cmd) == "string" then
        if cmd:match("^playerctl") then
            click_action = { Exec = cmd }
        else
            click_action = { ScriptCall = cmd }
        end
    else
        click_action = cmd
    end

    return {
        type = "flex",
        style = { 
            justify = "center", 
            align_items = "center", 
            padding = { top = 4, bottom = 4, left = 8, right = 8 } 
        },
        background = "#313244",
        radius = 4,
        on_click = click_action,
        tooltip = ttip,
        children = {
            { type = "text", text = icon, color = color or text_color }
        }
    }
end

function render(monitor)
    local p, bus_name = get_active_player()
    if not p then
        return { type = "flex" }
    end

    local title = p.track_name or "Unknown"
    local artist = p.artist or ""
    local is_playing = (p.status == "playing")
    
    local display_text = title
    if artist ~= "" then
        display_text = artist .. " — " .. title
    end

    local children = {}

    -- Player cycling: left arrow
    if #player_list > 1 then
        local prev_name = prev_player_name()
        table.insert(children, create_button("󰅁", "cycle_prev", prev_name, text_color))
    end

    -- Prev Track button
    table.insert(children, create_button("󰒮", "playerctl --player=" .. bus_name .. " previous", "Previous Track", text_color))

    -- Play/Pause button
    local status_icon = is_playing and "󰏤" or "󰐊"
    table.insert(children, create_button(status_icon, "playerctl --player=" .. bus_name .. " play-pause", is_playing and "Pause" or "Play", accent_color))

    -- Next Track button
    table.insert(children, create_button("󰒭", "playerctl --player=" .. bus_name .. " next", "Next Track", text_color))

    -- Player cycling: right arrow
    if #player_list > 1 then
        local nxt = next_player_name()
        table.insert(children, create_button("󰅂", "cycle_next", nxt, text_color))
    end

    local max_width = 30
    local truncated_text = display_text
    if #display_text > max_width then
        truncated_text = display_text:sub(1, max_width - 3) .. "..."
    end

    -- Track info
    table.insert(children, {
        type = "flex",
        style = { justify = "center", align_items = "center", padding = { top = 4, bottom = 4, left = 8, right = 8 } },
        background = "#1e1e2e",
        radius = 4,
        tooltip = {
            type = "flex",
            background = "#1e1e2eff",
            radius = 4,
            style = { padding = { top = 8, bottom = 8, left = 12, right = 12 }, gap = 4, direction = "column" },
            children = {
                { type = "text", text = "Title: " .. title, color = "#ffffff" },
                { type = "text", text = "Artist: " .. (artist ~= "" and artist or "Unknown"), color = text_color },
                { type = "text", text = "Status: " .. p.status, color = text_color },
                { type = "text", text = "Player: " .. (p.name or bus_name), color = accent_color },
            }
        },
        children = {
            { type = "text", text = truncated_text, color = text_color }
        }
    })

    return {
        type = "flex",
        style = { gap = 6, align_items = "center" },
        children = children,
    }
end
