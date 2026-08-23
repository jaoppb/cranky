local player_map = {}
local player_list = {}
local active_idx = 1

function init() end

function metadata()
	return {
		subscriptions = { "mpris" },
		styles = { "mpris" },
	}
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
		if active_idx > #player_list then
			active_idx = #player_list
		end
		if active_idx < 1 then
			active_idx = 1
		end
	else
		player_map = {}
		player_list = {}
		active_idx = 1
	end
end

-- ScriptCall callbacks for player cycling
function cycle_prev()
	if #player_list <= 1 then
		return
	end
	active_idx = active_idx - 1
	if active_idx < 1 then
		active_idx = #player_list
	end
end

function cycle_next()
	if #player_list <= 1 then
		return
	end
	active_idx = active_idx + 1
	if active_idx > #player_list then
		active_idx = 1
	end
end

local function get_active_player()
	if #player_list == 0 then
		return nil
	end
	local bus_name = player_list[active_idx]
	return player_map[bus_name], bus_name
end

local function get_player_display_name(idx)
	local bus = player_list[idx]
	local p = player_map[bus]
	return (p and p.name) or bus
end

local function prev_player_name()
	if #player_list <= 1 then
		return nil
	end
	local idx = active_idx - 1
	if idx < 1 then
		idx = #player_list
	end
	return get_player_display_name(idx)
end

local function next_player_name()
	if #player_list <= 1 then
		return nil
	end
	local idx = active_idx + 1
	if idx > #player_list then
		idx = 1
	end
	return get_player_display_name(idx)
end

local function create_button(icon, cmd, tooltip_text, class_name)
	local ttip = nil
	if tooltip_text then
		ttip = {
			type = "flex",
			class = "tooltip",
			children = {
				{ type = "text", text = tooltip_text },
			},
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
		class = class_name or "btn",
		on_click = click_action,
		tooltip = ttip,
		children = {
			{ type = "text", text = icon },
		},
	}
end

function render(monitor)
	local p, bus_name = get_active_player()
	if not p then
		return { type = "flex", class = "container" }
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
		table.insert(children, create_button("󰅁", "cycle_prev", prev_name, "btn-cycle"))
	end

	-- Prev Track button
	table.insert(
		children,
		create_button("󰒮", "playerctl --player=" .. bus_name .. " previous", "Previous Track", "btn-prev")
	)

	-- Play/Pause button
	local status_icon = is_playing and "󰏤" or "󰐊"
	table.insert(
		children,
		create_button(
			status_icon,
			"playerctl --player=" .. bus_name .. " play-pause",
			is_playing and "Pause" or "Play",
			"btn-play"
		)
	)

	-- Next Track button
	table.insert(
		children,
		create_button("󰒭", "playerctl --player=" .. bus_name .. " next", "Next Track", "btn-next")
	)

	-- Player cycling: right arrow
	if #player_list > 1 then
		local nxt = next_player_name()
		table.insert(children, create_button("󰅂", "cycle_next", nxt, "btn-cycle"))
	end

	local max_width = 30
	local truncated_text = display_text
	if #display_text > max_width then
		truncated_text = display_text:sub(1, max_width - 3) .. "..."
	end

	-- Track info
	table.insert(children, {
		type = "flex",
		class = "track-info",
		tooltip = {
			type = "flex",
			class = "tooltip",
			children = {
				{ type = "text", class = "title", text = "Title: " .. title },
				{ type = "text", class = "artist", text = "Artist: " .. (artist ~= "" and artist or "Unknown") },
				{ type = "text", class = "status", text = "Status: " .. p.status },
				{ type = "text", class = "player", text = "Player: " .. (p.name or bus_name) },
			},
		},
		children = {
			{ type = "text", text = truncated_text },
		},
	})

	return {
		type = "flex",
		class = "container",
		children = children,
	}
end
