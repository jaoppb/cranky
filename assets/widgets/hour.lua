local format = "%H:%M:%S"
local time_str = ""
local on_click = nil

function init()
	if config.format then
		format = config.format
	end
	if config.on_click then
		on_click = { Exec = config.on_click }
	end
end

function metadata()
	return {
		subscriptions = { "time" },
		styles = { "hour" },
	}
end

function refresh()
	if current_time then
		local y, m, d, h, min, s = current_time:match("^(%d+)%-(%d+)%-(%d+)T(%d+):(%d+):(%d+)")
		if y then
			local ts = os.time({ year = y, month = m, day = d, hour = h, min = min, sec = s })
			time_str = os.date(format, ts)
		else
			time_str = current_time
		end
	else
		time_str = os.date(format)
	end
end

function render(monitor)
	return vdom.text({
		class = "time",
		text = time_str,
		on_click = on_click,
	})
end
