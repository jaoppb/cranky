local show_titles = true
local show_icons = true
local max_items = 6
local empty_label = "systray: none"

local items = {}

function init()
	local options = config.module or config.options or {}
	if options.show_titles ~= nil then
		show_titles = options.show_titles
	end
	if options.show_icons ~= nil then
		show_icons = options.show_icons
	end
	if options.max_items ~= nil then
		max_items = options.max_items
	end
	if options.empty_label ~= nil then
		empty_label = options.empty_label
	end
end

function metadata()
	return {
		subscriptions = { "systray" },
		styles = { "systray" },
	}
end

function refresh()
	items = signals.systray or {}
end

function render(monitor)
	if #items == 0 then
		return ui.text({
			class = "empty",
			text = empty_label,
			tooltip = ui.flex({
				class = "tooltip",
				children = {
					ui.text({ text = "No systray items are currently active" }),
				},
			}),
		})
	end

	local children = {}
	for i, item in ipairs(items) do
		if i > max_items then
			break
		end

		local item_children = {}

		if show_icons then
			local img = nil
			if item.icon and type(item.icon) == "table" then
				img = item.icon.image
			end
			if type(img) == "table" then
				table.insert(
					item_children,
					ui.image({
						class = "icon",
						data = img.data,
						pixel_size = img.size,
					})
				)
			else
				table.insert(
					item_children,
					ui.rect({
						class = "icon-placeholder",
					})
				)
			end
		end

		if show_titles then
			local label = (item.title and item.title ~= "") and item.title or item.item_id or "app"
			table.insert(
				item_children,
				ui.text({
					class = "title",
					text = label,
				})
			)
		end

		local tooltip_children = {}
		if item.tooltip and type(item.tooltip) == "table" then
			local t = item.tooltip.title or ""
			local d = item.tooltip.description or ""
			if t ~= "" then
				table.insert(tooltip_children, ui.text({ class = "tooltip-title", text = t }))
			end
			if d ~= "" and d ~= t then
				table.insert(tooltip_children, ui.text({ class = "tooltip-desc", text = d }))
			end
		end
		if #tooltip_children == 0 then
			table.insert(
				tooltip_children,
				ui.text({
					class = "tooltip-title",
					text = (item.title and item.title ~= "") and item.title or item.item_id or "app",
				})
			)
		end

		local item_node = ui.flex({
			class = "item",
			key = tostring(item.id),
			children = item_children,
			on_click = {
				left = ui.action.systray(item.id, "Primary"),
				right = ui.action.systray(item.id, "ContextMenu"),
			},
			tooltip = ui.flex({
				class = "tooltip",
				children = tooltip_children,
			}),
		})
		table.insert(children, item_node)
	end

	return ui.flex({
		class = "container",
		children = children,
	})
end
