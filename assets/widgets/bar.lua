local left = {}
local center = {}
local right = {}

function init()
	local options = config.module or config.options or {}
	if options.left then
		left = options.left
	end
	if options.center then
		center = options.center
	end
	if options.right then
		right = options.right
	end
end

function metadata()
	return {
		subscriptions = {},
		styles = { "bar" },
	}
end

function refresh() end

function render(monitor)
	local left_children = {}
	for _, mod_name in ipairs(left) do
		table.insert(left_children, ui.module(mod_name))
	end

	local center_children = {}
	for _, mod_name in ipairs(center) do
		table.insert(center_children, ui.module(mod_name))
	end

	local right_children = {}
	for _, mod_name in ipairs(right) do
		table.insert(right_children, ui.module(mod_name))
	end

	return ui.flex({
		class = "root",
		children = {
			ui.flex({
				class = "left",
				children = left_children,
			}),
			ui.flex({
				class = "center",
				children = center_children,
			}),
			ui.flex({
				class = "right",
				children = right_children,
			}),
		},
	})
end
