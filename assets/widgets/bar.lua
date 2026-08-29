local left = {}
local center = {}
local right = {}

function init()
	if config.left then
		left = config.left
	end
	if config.center then
		center = config.center
	end
	if config.right then
		right = config.right
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
		table.insert(
			left_children,
			vdom.module({
				name = mod_name,
			})
		)
	end

	local center_children = {}
	for _, mod_name in ipairs(center) do
		table.insert(
			center_children,
			vdom.module({
				name = mod_name,
			})
		)
	end

	local right_children = {}
	for _, mod_name in ipairs(right) do
		table.insert(
			right_children,
			vdom.module({
				name = mod_name,
			})
		)
	end

	return vdom.flex({
		class = "root",
		children = {
			vdom.flex({
				class = "left",
				children = left_children,
			}),
			vdom.flex({
				class = "center",
				children = center_children,
			}),
			vdom.flex({
				class = "right",
				children = right_children,
			}),
		},
	})
end
