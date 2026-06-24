-- Metrics built-in module for Cranky

local state = {
    cpu = 0.0,
    ram_used = 0,
    ram_total = 0,
    net_tx = 0,
    net_rx = 0,
    temp = 0,
    disks = {},
    config = nil
}

function init()
end

function subscriptions()
    return { "metrics" }
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

-- Helper to format bytes
local function format_bytes(bytes)
    local units = {"B", "KB", "MB", "GB", "TB"}
    local i = 1
    local amount = bytes
    while amount >= 1024 and i < #units do
        amount = amount / 1024
        i = i + 1
    end
    return string.format("%.1f %s", amount, units[i])
end

local function get_color(percent)
    if percent < 50 then
        return "#a6e3a1" -- green
    elseif percent < 80 then
        return "#f9e2af" -- yellow
    else
        return "#f38ba8" -- red
    end
end

local function is_enabled(metric_name, global_mode)
    if config and config[metric_name] == false then
        return false
    end
    if global_mode == "disabled" then
        return false
    end
    return true
end

local function get_widgets()
    local widgets = {}
    if not state.config then return widgets end

    -- CPU
    if is_enabled("cpu", state.config.cpu) then
        table.insert(widgets, { type = "bar", label = "CPU", value = state.cpu, max = 100 })
    end

    -- Memory
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

    -- Disks
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

    -- Network
    if state.config.network and is_enabled("network", state.config.network) then
        table.insert(widgets, { type = "text", label = "NET", text = string.format("▼%s ▲%s", format_bytes(state.net_rx), format_bytes(state.net_tx)) })
    end

    -- Temperature
    if state.config.temperature and is_enabled("temperature", state.config.temperature) then
        local unit = state.config.temperature == "celsius" and "°C" or "°F"
        table.insert(widgets, { type = "text", label = "TMP", text = string.format("%.1f%s", state.temp, unit) })
    end

    return widgets
end

local padding = 10
local spacing = 15
local bar_width = 50
local bar_height = 6
local bar_radius = 3

function measure(canvas, monitor)
    local widgets = get_widgets()
    if #widgets == 0 then
        local w, h = canvas:measure_text("Loading metrics...")
        return math.ceil(w + padding * 2), h
    end

    local total_width = padding * 2
    local max_height = 0

    for i, w in ipairs(widgets) do
        local lw, lh = canvas:measure_text(w.label)
        local widget_width = lw + 5 -- label + spacing
        
        if w.type == "bar" then
            widget_width = widget_width + bar_width
        elseif w.type == "text" then
            local tw, th = canvas:measure_text(w.text)
            widget_width = widget_width + tw
        end

        total_width = total_width + widget_width
        if i < #widgets then
            total_width = total_width + spacing
        end

        if lh > max_height then max_height = lh end
    end

    return math.ceil(total_width), max_height
end

function view(canvas, monitor)
    local widgets = get_widgets()
    if #widgets == 0 then
        canvas:draw_text("Loading metrics...", "#cccccc", padding, 0)
        return
    end

    local x = padding
    local text_color = "#cccccc"
    local bg_color = "#313244" -- subtle background for the progress bar
    local text_font_size = bar_config and bar_config.font_size or nil

    for i, w in ipairs(widgets) do
        -- Draw label
        local lw, lh = canvas:measure_text(w.label)
        canvas:draw_text(w.label, text_color, x, 0)
        x = x + lw + 5

        if w.type == "bar" then
            -- Draw background bar
            local y_offset = (lh - bar_height) / 2
            canvas:draw_rect(x, y_offset, bar_width, bar_height, bg_color, bar_radius)
            
            -- Draw foreground bar
            local percent = math.min(100, math.max(0, w.value))
            local fg_width = (percent / 100) * bar_width
            if fg_width > 0 then
                local fg_color = get_color(percent)
                -- We only round the right side, so we draw it over the background
                canvas:draw_rect(x, y_offset, fg_width, bar_height, fg_color, bar_radius)
            end
            
            x = x + bar_width
        elseif w.type == "text" then
            canvas:draw_text(w.text, text_color, x, 0)
            local tw, th = canvas:measure_text(w.text)
            x = x + tw
        end

        if i < #widgets then
            x = x + spacing
        end
    end
end
