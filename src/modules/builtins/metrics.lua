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

function on_event(event)
    if event.type == "metrics" then
        if event.metrics then
            state.cpu = event.metrics.cpu_usage
            state.ram_used = event.metrics.memory_used
            state.ram_total = event.metrics.memory_total
            state.net_tx = event.metrics.network_tx
            state.net_rx = event.metrics.network_rx
            state.temp = event.metrics.temperature
            state.disks = event.metrics.disks
            state.config = event.metrics.config
        end
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

-- Renders the metrics based on user configuration
local function get_metrics_text()
    if not state.config then
        return "Loading metrics..."
    end

    local texts = {}

    -- CPU
    table.insert(texts, string.format("CPU: %.1f%%", state.cpu))

    -- Memory
    if state.config.memory == "absolute" then
        table.insert(texts, string.format("RAM: %s", format_bytes(state.ram_used)))
    else
        local percent = 0
        if state.ram_total > 0 then
            percent = (state.ram_used / state.ram_total) * 100
        end
        table.insert(texts, string.format("RAM: %.1f%%", percent))
    end

    -- Disks (just sum up root or first disk as example)
    if state.config.disk and #state.disks > 0 then
        local d = state.disks[1]
        if state.config.disk == "absolute" then
            table.insert(texts, string.format("Disk: %s", format_bytes(d.used_bytes)))
        else
            local percent = 0
            if d.total_bytes > 0 then
                percent = (d.used_bytes / d.total_bytes) * 100
            end
            table.insert(texts, string.format("Disk: %.1f%%", percent))
        end
    end

    -- Network
    if state.config.network then
        table.insert(texts, string.format("▼%s ▲%s", format_bytes(state.net_rx), format_bytes(state.net_tx)))
    end

    -- Temperature
    if state.config.temperature then
        local unit = state.config.temperature == "celsius" and "°C" or "°F"
        table.insert(texts, string.format("Temp: %.1f%s", state.temp, unit))
    end

    return table.concat(texts, " | ")
end

function measure(canvas, monitor)
    local text = get_metrics_text()
    local w, h = canvas:measure_text(text)
    -- Add 10px padding on left/right
    return math.ceil(w + 20), h
end

function view(canvas, monitor)
    local text = get_metrics_text()
    local w, h = canvas:measure_text(text)
    
    local text_color = "#cccccc"
    canvas:draw_text(text, text_color, 10, 4)
end
