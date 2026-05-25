local format = "%H:%M:%S"
local time_str = ""

function init()
    if config.format then
        format = config.format
    end
end

function subscriptions()
    return { "time" }
end

function refresh()
    -- current_time is passed from Rust as RFC3339, but we can also just use os.date
    -- if we want to follow the format exactly.
    -- However, os.date uses C format strings which are mostly compatible with chrono.
    time_str = os.date(format)
end

function measure(canvas, monitor)
    return canvas:measure_text(time_str, bar_config.font_family, bar_config.font_size)
end

function view(canvas, monitor)
    local color = "#c0caf5"
    canvas:draw_text(time_str, bar_config.font_family, bar_config.font_size, color, 0, 0)
end

function on_event(event)
    if event.type == "click" and event.button == 272 then
        if format == "%H:%M:%S" then
            format = "%I:%M:%S %p"
        else
            format = "%H:%M:%S"
        end
        time_str = os.date(format)
    end
end
