local workspaces = {}
local active_workspaces = {}
local focused_monitor = ""

function init() end

function metadata()
	return {
		subscriptions = { "hyprland" },
		styles = { "workspace" },
	}
end

function refresh()
	if not hyprland then
		return
	end

	workspaces = hyprland.workspaces
	focused_monitor = hyprland.focused_monitor or ""
	active_workspaces = {}
	for _, m in ipairs(hyprland.monitors) do
		active_workspaces[m.name] = {
			active = m.active_workspace_id,
			special = m.special_workspace_id,
		}

		local found = false
		for _, ws in ipairs(workspaces) do
			if ws.id == m.active_workspace_id then
				found = true
				break
			end
		end
		if not found then
			table.insert(workspaces, {
				id = m.active_workspace_id,
				name = tostring(m.active_workspace_id),
				monitor = m.name,
			})
		end

		if type(m.special_workspace_id) == "number" and m.special_workspace_id ~= 0 then
			local found_sp = false
			for _, ws in ipairs(workspaces) do
				if ws.id == m.special_workspace_id then
					found_sp = true
					break
				end
			end
			if not found_sp then
				table.insert(workspaces, {
					id = m.special_workspace_id,
					name = "special",
					monitor = m.name,
				})
			end
		end
	end

	table.sort(workspaces, function(a, b)
		if a.id > 0 and b.id > 0 then
			return a.id < b.id
		end
		if a.id < 0 and b.id < 0 then
			return a.id > b.id
		end
		return a.id > b.id
	end)
end

function render(monitor)
	local monitor_id = monitor:id()
	local active_ids = active_workspaces[monitor_id] or { active = -1 }

	local children = {}

	for _, ws in ipairs(workspaces) do
		if ws.monitor == monitor_id then
			local is_special = ws.name:match("^special:") or (ws.id < 0)
			local label = ws.name:match("^special:(.*)") or ws.name
			if label == "" or label == tostring(ws.id) then
				if is_special then
					label = "special"
				end
			end

			local active_ws = (type(active_ids.special) == "number" and active_ids.special ~= 0) and active_ids.special
				or active_ids.active
			local is_visible = (ws.id == active_ws)

			local exec_cmd
			if is_special then
				exec_cmd = "hyprctl dispatch 'hl.dsp.workspace.toggle_special(\"" .. label .. "\")'"
			else
				exec_cmd = "hyprctl dispatch 'hl.dsp.focus({ workspace = \"" .. ws.id .. "\"})'"
			end

			local text_node = vdom.text({
				class = "label",
				text = label,
			})

			local classes = "item"
			if is_visible then
				if focused_monitor == monitor_id then
					classes = "item active focused"
				else
					classes = "item active"
				end
			end

			table.insert(
				children,
				vdom.flex({
					class = classes,
					id = "ws-" .. ws.id,
					on_click = { Exec = exec_cmd },
					tooltip = vdom.flex({
						class = "tooltip",
						children = {
							vdom.text({ text = "Switch to workspace " .. label }),
						},
					}),
					children = { text_node },
				})
			)
		end
	end

	return vdom.flex({
		class = "container",
		children = children,
	})
end
