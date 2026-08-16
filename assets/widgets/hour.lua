local format = "%H:%M:%S"
local time_str = ""
local color = "#c0caf5"
local font = nil
local size = nil
local tooltip = nil
local on_click = nil

function init()
    if config.format then format = config.format end
    if config.color then color = config.color end
    if config.font then font = config.font end
    if config.size then size = config.size end
    if config.tooltip then tooltip = config.tooltip end
    if config.on_click then on_click = { Exec = config.on_click } end
end

function subscriptions()
    return { "time" }
end

function refresh()
    if current_time then
        local y, m, d, h, min, s = current_time:match("^(%d+)%-(%d+)%-(%d+)T(%d+):(%d+):(%d+)")
        if y then
            local ts = os.time({year=y, month=m, day=d, hour=h, min=min, sec=s})
            time_str = os.date(format, ts)
        else
            time_str = current_time
        end
    else
        time_str = os.date(format)
    end
end

function render(monitor)
    return {
        type = "text",
        text = time_str,
        color = color,
        font = font,
        size = size,
        tooltip = tooltip,
        on_click = on_click,
    }
end
