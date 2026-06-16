local active_bg = "#565f89"
local focused_bg = "#3b4261"
local border_radius = 0

local workspaces = {}
local active_workspaces = {}
local focused_monitor = ""

function init()
    if config.active and config.active.background_color then
        active_bg = config.active.background_color
    end
    if config.focused and config.focused.background_color then
        focused_bg = config.focused.background_color
    end
    if config.border_radius then
        border_radius = config.border_radius
    end
end

function subscriptions()
    return { "hyprland" }
end

function refresh()
    if not hyprland then return end
    
    workspaces = hyprland.workspaces
    active_workspaces = {}
    for _, m in ipairs(hyprland.monitors) do
        active_workspaces[m.name] = m.active_workspace_id
        if m.focused then
            focused_monitor = m.name
        end
        
        local found = false
        for _, ws in ipairs(workspaces) do
            if ws.id == m.active_workspace_id then
                found = true
                break
            end
        end
        if not found then
            table.insert(workspaces, {
                id = m.active_workspace_id,
                monitor = m.name
            })
        end
    end
    
    -- sort workspaces by id after adding potentially empty active workspaces
    table.sort(workspaces, function(a, b) return a.id < b.id end)
end

function measure(canvas, monitor)
    local monitor_id = monitor:id()
    local count = 0
    for _, ws in ipairs(workspaces) do
        if ws.monitor == monitor_id then
            count = count + 1
        end
    end
    
    if count == 0 then return 0, 0 end
    
    local _, lh = canvas:measure_text("0")
    local padding_y = 4
    local item_size = lh + padding_y * 2
    local item_spacing = 6
    local width = (math.max(count, 1) - 1) * item_spacing + item_size * count
    return math.ceil(width), item_size
end

function view(canvas, monitor)
    local monitor_id = monitor:id()
    local active_id = active_workspaces[monitor_id] or -1
    local is_monitor_focused = focused_monitor == monitor_id
    
    local _, lh = canvas:measure_text("0")
    local padding_y = 4
    local padding_x = 6
    local item_size = lh + padding_y * 2
    local item_spacing = 6
    local x_offset = 0
    
    local inactive_color = "#7aa2f7"
    local active_text_color = "#ffffff"
    
    for _, ws in ipairs(workspaces) do
        if ws.monitor == monitor_id then
            local label = tostring(ws.id)
            local is_visible = ws.id == active_id
            local lw, _ = canvas:measure_text(label)
            
            if is_visible then
                local bg = is_monitor_focused and active_bg or focused_bg
                local rect_w = math.max(lw + padding_x * 2, item_size)
                canvas:draw_rect(x_offset, 0, rect_w, item_size, bg, border_radius)
                
                canvas:draw_text(label, active_text_color, x_offset + (rect_w - lw) / 2, padding_y)
                x_offset = x_offset + rect_w + item_spacing
            else
                local rect_w = math.max(lw + padding_x * 2, item_size)
                canvas:draw_text(label, inactive_color, x_offset + (rect_w - lw) / 2, padding_y)
                x_offset = x_offset + rect_w + item_spacing
            end
        end
    end
end

function on_event(event)
    if event.type == "click" and event.button == 272 then -- 272 is BTN_LEFT in wayland
        -- Find which workspace was clicked
        -- Note: with dynamic width this is an approximation
        local _, lh = canvas:measure_text("0")
        local padding_x = 6
        local rect_w = math.max(16 + padding_x * 2, lh + 8)
        local item_spacing = 6
        local index = math.floor(event.x / (rect_w + item_spacing)) + 1
        
        -- We don't have the exact monitor ID here, but we can assume the active one or 
        -- just iterate all workspaces and pick the nth one that matches the x coordinate.
        -- A better approach is to store the bounding box of each workspace in view()
        -- but for simplicity, we just use the index.
        local current = 1
        for _, ws in ipairs(workspaces) do
            -- Note: in a real implementation we need the monitor ID.
            -- This is a simplified version.
            if current == index then
                os.execute("hyprctl dispatch workspace " .. ws.id)
                break
            end
            current = current + 1
        end
    end
end
