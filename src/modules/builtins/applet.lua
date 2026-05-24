local show_titles = true
local show_icons = true
local icon_size = 16
local max_items = 6
local empty_label = "applet: none"
local font_family = ""
local font_size = 14

local items = {}
local error_message = nil

function init()
    if config.show_titles ~= nil then show_titles = config.show_titles end
    if config.show_icons ~= nil then show_icons = config.show_icons end
    if config.icon_size ~= nil then icon_size = config.icon_size end
    if config.max_items ~= nil then max_items = config.max_items end
    if config.empty_label ~= nil then empty_label = config.empty_label end
    font_family = bar_config.font_family
    font_size = bar_config.font_size
end

function refresh()
    -- For now, applet list is empty in Lua too until we expose DBus
    items = {}
end

function measure(canvas, monitor)
    local text_color = "#c0caf5"
    if #items == 0 then
        local w, h = canvas:measure_text(empty_label, font_family, font_size)
        return math.ceil(w), math.ceil(h)
    end

    local total_w = 0
    local ITEM_SPACING = 8
    local ICON_TEXT_GAP = 6

    for i, item in ipairs(items) do
        if i > max_items then break end
        if i > 1 then total_w = total_w + ITEM_SPACING end
        if show_icons then total_w = total_w + icon_size + ICON_TEXT_GAP end
        if show_titles then
            local label = item.title or item.app_id or "app"
            local w, _ = canvas:measure_text(label, font_family, font_size)
            total_w = total_w + w
        end
    end
    return math.ceil(total_w), 30
end

function view(canvas, monitor)
    local text_color = "#c0caf5"
    local x = 0
    
    if error_message then
        canvas:draw_text("error: " .. error_message, font_family, font_size, text_color, 0, 0)
        return
    end

    if #items == 0 then
        canvas:draw_text(empty_label, font_family, font_size, text_color, 0, 0)
        return
    end

    local ITEM_SPACING = 8
    local ICON_TEXT_GAP = 6

    for i, item in ipairs(items) do
        if i > max_items then break end
        if i > 1 then x = x + ITEM_SPACING end
        
        if show_icons then
            canvas:draw_rect(x, 0, icon_size, icon_size, text_color, 2)
            x = x + icon_size + ICON_TEXT_GAP
        end

        if show_titles then
            local label = item.title or item.app_id or "app"
            local lw, lh = canvas:measure_text(label, font_family, font_size)
            canvas:draw_text(label, font_family, font_size, text_color, x, 0)
            x = x + lw
        end
    end
end
