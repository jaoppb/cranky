local show_titles = true
local show_icons = true
local max_items = 6
local empty_label = "systray: none"

local items = {}

function init()
	if config.show_titles ~= nil then
		show_titles = config.show_titles
	end
	if config.show_icons ~= nil then
		show_icons = config.show_icons
	end
	if config.max_items ~= nil then
		max_items = config.max_items
	end
	if config.empty_label ~= nil then
		empty_label = config.empty_label
	end
end

function metadata()
	return {
		subscriptions = { "systray" },
		styles = { "systray" },
	}
end

function refresh()
	items = systray or {}
end

function render(monitor)
	if #items == 0 then
		return {
			type = "text",
			class = "empty",
			text = empty_label,
			tooltip = {
				type = "flex",
				class = "tooltip",
				children = {
					{ type = "text", text = "No systray items are currently active" },
				},
			},
		}
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
				table.insert(item_children, {
					type = "image",
					class = "icon",
					data = img.data,
					pixel_size = img.size,
				})
			else
				table.insert(item_children, {
					type = "rect",
					class = "icon-placeholder",
				})
			end
		end

		if show_titles then
			local label = (item.title and item.title ~= "") and item.title or item.item_id or "app"
			table.insert(item_children, {
				type = "text",
				class = "title",
				text = label,
			})
		end

		local tooltip_children = {}
		if item.tooltip and type(item.tooltip) == "table" then
			local t = item.tooltip.title or ""
			local d = item.tooltip.description or ""
			if t ~= "" then
				table.insert(tooltip_children, { type = "text", class = "tooltip-title", text = t })
			end
			if d ~= "" and d ~= t then
				table.insert(tooltip_children, { type = "text", class = "tooltip-desc", text = d })
			end
		end
		if #tooltip_children == 0 then
			table.insert(tooltip_children, {
				type = "text",
				class = "tooltip-title",
				text = (item.title and item.title ~= "") and item.title or item.item_id or "app",
			})
		end

		local item_node = {
			type = "flex",
			class = "item",
			children = item_children,
			on_click = {
				SystrayAction = {
					id = item.id,
					action = "Primary",
				},
			},
			tooltip = {
				type = "flex",
				class = "tooltip",
				children = tooltip_children,
			},
		}
		table.insert(children, item_node)
	end

	return {
		type = "flex",
		class = "container",
		children = children,
	}
end
