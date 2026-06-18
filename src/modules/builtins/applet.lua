local show_titles = true
local show_icons = true
local icon_size = 16
local max_items = 6
local empty_label = "applet: none"

local items = {}
local error_message = nil

function init()
    if config.show_titles ~= nil then show_titles = config.show_titles end
    if config.show_icons ~= nil then show_icons = config.show_icons end
    if config.icon_size ~= nil then icon_size = config.icon_size end
    if config.max_items ~= nil then max_items = config.max_items end
    if config.empty_label ~= nil then empty_label = config.empty_label end
end

function subscriptions()
    return { "applets" }
end

function refresh()
    items = {}
    if _G.applets then
        for _, item in ipairs(_G.applets) do
            table.insert(items, item)
        end
    end
end

function measure(canvas, monitor)
    local text_color = "#c0caf5"
    if #items == 0 then
        local w, h = canvas:measure_text(empty_label)
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
            local w, _ = canvas:measure_text(label)
            total_w = total_w + w
        end
    end
    return math.ceil(total_w), 30
end

function view(canvas, monitor)
    local text_color = "#c0caf5"
    local x = 0
    
    if error_message then
        canvas:draw_text("error: " .. error_message, text_color, 0, 0)
        return
    end

    if #items == 0 then
        canvas:draw_text(empty_label, text_color, 0, 0)
        return
    end

    local ITEM_SPACING = 8
    local ICON_TEXT_GAP = 6

    for i, item in ipairs(items) do
        if i > max_items then break end
        if i > 1 then x = x + ITEM_SPACING end
        if type(item) == "table" then
            local mt = getmetatable(item)
            if mt then error("ITEM HAS METATABLE: " .. type(mt.__index)) end
        end
        if type(item) == "userdata" then error("ITEM IS USERDATA") end
        
        if show_icons then
            if type(item.icon_image) == "table" and type(item.icon_image.data) == "table" and item.icon_image.size then
                canvas:draw_image(item.icon_image.data, item.icon_image.size.width, item.icon_image.size.height, icon_size, icon_size, x, (30 - icon_size) / 2)
            else
                canvas:draw_rect(x, (30 - icon_size) / 2, icon_size, icon_size, text_color, 2)
            end
            x = x + icon_size + ICON_TEXT_GAP
        end

        if show_titles then
            local label = item.title or item.app_id or "app"
            local lw, lh = canvas:measure_text(label)
            canvas:draw_text(label, text_color, x, (30 - lh) / 2)
            x = x + lw
        end
    end
end

local BUTTON_ACTIONS = {
    [272] = "Primary",           -- Left Click: try ContextMenu, fallback to Activate
    [273] = "ContextMenu",       -- Right Click
    [274] = "SecondaryActivate"  -- Middle Click
}

local SCROLL_ACTIONS = {
    ["up"] = "ScrollUp",
    ["down"] = "ScrollDown",
    ["left"] = "ScrollLeft",
    ["right"] = "ScrollRight"
}

function on_event(event)
    if event.type == "pointer_leave" then
        if _G.cranky and _G.cranky.hide_tooltip then
            _G.cranky.hide_tooltip()
        end
        return
    end

    local action = nil
    local is_motion = false
    
    if event.type == "click" then
        action = BUTTON_ACTIONS[event.button]
    elseif event.type == "scroll" then
        action = SCROLL_ACTIONS[event.direction]
    elseif event.type == "motion" then
        is_motion = true
    end
    
    if not action and not is_motion then return end

    local ITEM_SPACING = 8
    local ICON_TEXT_GAP = 6
    local x = 0
    
    for i, item in ipairs(items) do
        if i > max_items then break end
        if i > 1 then x = x + ITEM_SPACING end
        
        local item_width = 0
        if show_icons then item_width = item_width + icon_size + ICON_TEXT_GAP end
        if show_titles then
            local label = item.title or item.app_id or "app"
            item_width = item_width + (#label * 7) 
        end
        
        if event.x >= x and event.x <= x + item_width then
            if is_motion then
                if _G.cranky and _G.cranky.show_tooltip then
                    local title = item.title or item.app_id or "applet"
                    _G.cranky.show_tooltip(title)
                end
            elseif action then
                if _G.cranky and _G.cranky.applet_action then
                    _G.cranky.applet_action(item.id, action)
                end
            end
            return
        end
        x = x + item_width
    end

    if is_motion then
        if _G.cranky and _G.cranky.hide_tooltip then
            _G.cranky.hide_tooltip()
        end
    end
end
