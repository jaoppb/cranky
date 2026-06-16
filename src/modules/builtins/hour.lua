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
