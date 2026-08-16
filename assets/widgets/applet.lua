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

function render(monitor)
    local text_color = "#c0caf5"
    
    if error_message then
        return {
            type = "text",
            text = "error: " .. error_message,
            color = text_color
        }
    end

    if #items == 0 then
        return {
            type = "text",
            text = empty_label,
            color = text_color,
            tooltip = {
                type = "flex",
                background = "#1e1e2eff",
                radius = 4,
                style = { padding = { top = 4, bottom = 4, left = 8, right = 8 } },
                children = {
                    { type = "text", text = "No applets are currently active", color = text_color }
                }
            }
        }
    end

    local children = {}
    for i, item in ipairs(items) do
        if i > max_items then break end
        
        local item_children = {}
        
        if show_icons then
            local img = nil
            if item.icon and type(item.icon) == "table" then
                img = item.icon.image or (item.icon.data and item.icon)
            end
            if not img then
                img = item.icon_image
            end
            if type(img) == "table" then
                table.insert(item_children, {
                    type = "image",
                    size = { width = icon_size, height = icon_size },
                    data = img.data,
                    pixel_size = img.size
                })
            else
                table.insert(item_children, {
                    type = "rect",
                    size = { width = icon_size, height = icon_size },
                    color = text_color
                })
            end
        end
        
        if show_titles then
            local label = item.title or item.app_id or "app"
            table.insert(item_children, {
                type = "text",
                text = label,
                color = text_color
            })
        end
        
        local tooltip_children = {}
        if item.tooltip and type(item.tooltip) == "table" then
            local t = item.tooltip.title or ""
            local d = item.tooltip.description or ""
            if t ~= "" then
                table.insert(tooltip_children, { type = "text", text = t, color = "#c0caf5" })
            end
            if d ~= "" and d ~= t then
                table.insert(tooltip_children, { type = "text", text = d, color = "#c0caf5" })
            end
        end
        if #tooltip_children == 0 then
            table.insert(tooltip_children, {
                type = "text",
                text = item.title or item.app_id or "app",
                color = "#c0caf5"
            })
        end

        local applet_node = {
            type = "flex",
            style = { gap = 6, align_items = "center" },
            children = item_children,
            on_click = {
                AppletAction = {
                    id = item.id,
                    action = "Primary"
                }
            },
            tooltip = {
                type = "flex",
                style = { padding = { top = 6, bottom = 6, left = 10, right = 10 }, gap = 4, direction = "column" },
                background = "#1e1e2eff",
                radius = 6,
                children = tooltip_children
            }
        }
        table.insert(children, applet_node)
    end
    
    return {
        type = "flex",
        style = { gap = 8, align_items = "center" },
        children = children
    }
end
