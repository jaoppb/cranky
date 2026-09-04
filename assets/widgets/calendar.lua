local first_day_of_week = "sunday"
local today_year = 0
local today_month = 0
local today_day = 0
local view_year = 0
local view_month = 0

local month_names = {
	"January",
	"February",
	"March",
	"April",
	"May",
	"June",
	"July",
	"August",
	"September",
	"October",
	"November",
	"December",
}

local month_days = { 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 }

local function is_leap_year(y)
	return (y % 4 == 0 and y % 100 ~= 0) or (y % 400 == 0)
end

local function get_days_in_month(y, m)
	if m == 2 and is_leap_year(y) then
		return 29
	end
	return month_days[m]
end

-- Sakamoto's algorithm: 0 = Sunday, 1 = Monday, ..., 6 = Saturday
local function day_of_week(y, m, d)
	local t = { 0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4 }
	if m < 3 then
		y = y - 1
	end
	return (y + math.floor(y / 4) - math.floor(y / 100) + math.floor(y / 400) + t[m] + d) % 7
end

function init()
	local options = config.module or config.options or {}
	if options.first_day_of_week then
		first_day_of_week = string.lower(options.first_day_of_week)
	end
end

function metadata()
	return {
		subscriptions = { "time" },
		styles = { "calendar" },
	}
end

function prev_month()
	view_month = view_month - 1
	if view_month < 1 then
		view_month = 12
		view_year = view_year - 1
	end
end

function next_month()
	view_month = view_month + 1
	if view_month > 12 then
		view_month = 1
		view_year = view_year + 1
	end
end

function reset_today()
	view_year = today_year
	view_month = today_month
end

function refresh()
	if signals.time then
		local y, m, d = signals.time:match("^(%d+)%-(%d+)%-(%d+)")
		if y then
			local ny, nm, nd = tonumber(y), tonumber(m), tonumber(d)
			today_year, today_month, today_day = ny, nm, nd
			if view_year == 0 then
				view_year = ny
				view_month = nm
			end
		end
	end
end

function render(monitor)
	if view_year == 0 then
		view_year = 2026
		view_month = 1
	end

	-- Header with navigation buttons and month/year title
	local header_node = ui.flex({
		class = "header",
		children = {
			ui.text({
				class = "btn nav-btn",
				text = "<",
				on_click = ui.action.call("prev_month"),
			}),
			ui.text({
				class = "title",
				text = string.format("%s %d", month_names[view_month], view_year),
				on_click = ui.action.call("reset_today"),
			}),
			ui.text({
				class = "btn nav-btn",
				text = ">",
				on_click = ui.action.call("next_month"),
			}),
		},
	})

	-- Weekday headers
	local weekday_labels = (first_day_of_week == "monday")
			and { "Mo", "Tu", "We", "Th", "Fr", "Sa", "Su" }
		or { "Su", "Mo", "Tu", "We", "Th", "Fr", "Sa" }

	local weekday_children = {}
	for _, label in ipairs(weekday_labels) do
		table.insert(
			weekday_children,
			ui.text({
				class = "weekday",
				text = label,
			})
		)
	end

	local weekdays_node = ui.grid({
		class = "weekdays",
		children = weekday_children,
	})

	-- Days matrix calculation
	local first_dow = day_of_week(view_year, view_month, 1)
	local leading_count = (first_day_of_week == "monday") and ((first_dow + 6) % 7) or first_dow

	local prev_m = (view_month == 1) and 12 or (view_month - 1)
	local prev_y = (view_month == 1) and (view_year - 1) or view_year
	local prev_m_days = get_days_in_month(prev_y, prev_m)
	local cur_m_days = get_days_in_month(view_year, view_month)

	local day_children = {}

	-- Leading days (previous month)
	for i = leading_count - 1, 0, -1 do
		table.insert(
			day_children,
			ui.text({
				class = "day other-month",
				text = tostring(prev_m_days - i),
			})
		)
	end

	-- Current month days
	for d = 1, cur_m_days do
		local is_today = (view_year == today_year and view_month == today_month and d == today_day)
		local class_name = is_today and "day today" or "day"
		table.insert(
			day_children,
			ui.text({
				class = class_name,
				text = tostring(d),
			})
		)
	end

	-- Trailing days (next month) to fill 42 cells (6 rows x 7 cols)
	local total_so_far = #day_children
	local trailing_count = 42 - total_so_far
	for d = 1, trailing_count do
		table.insert(
			day_children,
			ui.text({
				class = "day other-month",
				text = tostring(d),
			})
		)
	end

	local days_grid_node = ui.grid({
		class = "grid",
		children = day_children,
	})

	return ui.flex({
		class = "root",
		children = {
			header_node,
			weekdays_node,
			days_grid_node,
		},
	})
end
