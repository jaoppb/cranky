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

function render(monitor)
    return {
        type = "text",
        text = time_str,
        color = "#c0caf5"
    }
end
