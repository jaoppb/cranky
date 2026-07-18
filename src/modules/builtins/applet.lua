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
            color = text_color
        }
    end

    local children = {}
    for i, item in ipairs(items) do
        if i > max_items then break end
        
        local item_children = {}
        
        if show_icons then
            if type(item.icon_image) == "table" then
                table.insert(item_children, {
                    type = "image",
                    size = { width = icon_size, height = icon_size },
                    data = item.icon_image.data,
                    pixel_size = item.icon_image.size
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
        
        local applet_node = {
            type = "flex",
            style = { gap = 6, align_items = "center" },
            children = item_children,
            on_click = {
                AppletAction = {
                    id = item.id,
                    action = "Primary"
                }
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
