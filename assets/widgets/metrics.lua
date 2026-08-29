local state = {
	cpu = 0.0,
	ram_used = 0,
	ram_total = 0,
	net_tx = 0,
	net_rx = 0,
	temp = 0,
	disks = {},
	config = nil,
}

function init() end

function metadata()
	return {
		subscriptions = { "metrics" },
		styles = { "metrics" },
	}
end

function refresh()
	if metrics then
		state.cpu = metrics.cpu_usage
		state.ram_used = metrics.memory_used
		state.ram_total = metrics.memory_total
		state.net_tx = metrics.network_tx
		state.net_rx = metrics.network_rx
		state.temp = metrics.temperature
		state.disks = metrics.disks
		state.config = metrics.config
	end
end

local function format_bytes(bytes)
	local units = { "B", "KB", "MB", "GB", "TB" }
	local i = 1
	local amount = bytes
	while amount >= 1024 and i < #units do
		amount = amount / 1024
		i = i + 1
	end
	return string.format("%.1f %s", amount, units[i])
end

local function get_progress_class(percent)
	if percent < 50 then
		return "progress"
	elseif percent < 80 then
		return "progress warning"
	else
		return "progress critical"
	end
end

local function is_enabled(metric_name, global_mode)
	if config and config[metric_name] == false then
		return false
	end
	if global_mode == nil or global_mode == "disabled" then
		return false
	end
	return true
end

local function get_widgets()
	local widgets = {}
	if not state.config then
		return widgets
	end

	if is_enabled("cpu", state.config.cpu) then
		table.insert(widgets, { type = "bar", label = "CPU", value = state.cpu, max = 100 })
	end

	if is_enabled("memory", state.config.memory) then
		if state.config.memory == "absolute" then
			table.insert(widgets, { type = "text", label = "RAM", text = format_bytes(state.ram_used) })
		else
			local percent = 0
			if state.ram_total > 0 then
				percent = (state.ram_used / state.ram_total) * 100
			end
			table.insert(widgets, { type = "bar", label = "RAM", value = percent, max = 100 })
		end
	end

	if state.config.disk and is_enabled("disk", state.config.disk) and #state.disks > 0 then
		local d = state.disks[1]
		if state.config.disk == "absolute" then
			table.insert(widgets, { type = "text", label = "DSK", text = format_bytes(d.used_bytes) })
		else
			local percent = 0
			if d.total_bytes > 0 then
				percent = (d.used_bytes / d.total_bytes) * 100
			end
			table.insert(widgets, { type = "bar", label = "DSK", value = percent, max = 100 })
		end
	end

	if state.config.network and is_enabled("network", state.config.network) then
		table.insert(
			widgets,
			{
				type = "text",
				label = "NET",
				text = string.format("▼%s ▲%s", format_bytes(state.net_rx), format_bytes(state.net_tx)),
			}
		)
	end

	if state.config.temperature and is_enabled("temperature", state.config.temperature) then
		local unit = state.config.temperature == "celsius" and "°C" or "°F"
		table.insert(widgets, { type = "text", label = "TMP", text = string.format("%.1f%s", state.temp, unit) })
	end

	return widgets
end

function render(monitor)
	if not state.config then
		return vdom.flex({ class = "container" })
	end

	local widgets = get_widgets()
	if #widgets == 0 then
		return vdom.flex({ class = "container" })
	end

	local children = {}

	for i, w in ipairs(widgets) do
		local widget_children = {}

		table.insert(
			widget_children,
			vdom.text({
				class = "label",
				text = w.label,
			})
		)

		if w.type == "bar" then
			local percent = math.min(100, math.max(0, w.value))
			table.insert(
				widget_children,
				vdom.progress({
					class = get_progress_class(percent),
					value = percent / 100.0,
					orientation = "horizontal",
				})
			)
		elseif w.type == "text" then
			table.insert(
				widget_children,
				vdom.text({
					class = "value",
					text = w.text,
				})
			)
		end

		table.insert(
			children,
			vdom.flex({
				class = "item",
				children = widget_children,
			})
		)
	end

	return vdom.flex({
		class = "container",
		children = children,
	})
end
