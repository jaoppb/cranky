local format = "%H:%M:%S"
local time_str = ""
local on_click = nil
local show_popups = {}

function init()
	local options = config.module or config.options or {}
	if options.format then
		format = options.format
	end
	if options.on_click then
		on_click = ui.action.exec(options.on_click)
	else
		on_click = ui.action.call("toggle_popup")
	end
end

function metadata()
	return {
		subscriptions = { "time" },
		styles = { "clock" },
	}
end

local function get_monitor_key(mon)
	if type(mon) == "string" then
		return mon
	elseif type(mon) == "table" or type(mon) == "userdata" then
		return mon.name or mon.id or tostring(mon)
	end
	return "default"
end

function toggle_popup(mon)
	if mon == nil or mon == "default" then
		show_popups["default"] = not show_popups["default"]
	else
		local key = get_monitor_key(mon)
		show_popups[key] = not show_popups[key]
	end
end

function on_popup_dismiss(mon)
	if mon == nil or mon == "default" then
		show_popups["default"] = false
		for k in pairs(show_popups) do
			show_popups[k] = false
		end
	else
		local key = get_monitor_key(mon)
		show_popups[key] = false
	end
end

function refresh()
	if signals.time then
		local y, m, d, h, min, s = signals.time:match("^(%d+)%-(%d+)%-(%d+)T(%d+):(%d+):(%d+)")
		if y then
			local ts = os.time({ year = y, month = m, day = d, hour = h, min = min, sec = s })
			time_str = os.date(format, ts)
		else
			time_str = signals.time
		end
	else
		time_str = os.date(format)
	end
end

function render(monitor)
	local node = {
		class = "time",
		text = time_str,
		on_click = on_click,
	}
	local key = get_monitor_key(monitor)
	local is_open = show_popups[key]
	if is_open == nil then
		is_open = show_popups["default"]
	end
	if is_open then
		node.popup = ui.popup({
			content = ui.module({ name = "calendar", class = "calendar" }),
			anchor = "bottom",
			dismiss_on_unfocus = true,
		})
	end
	return ui.text(node)
end
