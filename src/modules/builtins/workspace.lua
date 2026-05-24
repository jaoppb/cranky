local active_bg = "#565f89"
local focused_bg = "#3b4261"
local border_radius = 0
local font_family = ""
local font_size = 14

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
    font_family = bar_config.font_family
    font_size = bar_config.font_size
end

function refresh()
    if not hyprland then return end
    
    workspaces = hyprland.workspaces
    -- sort workspaces by id
    table.sort(workspaces, function(a, b) return a.id < b.id end)
    
    active_workspaces = {}
    for _, m in ipairs(hyprland.monitors) do
        active_workspaces[m.name] = m.activeWorkspace.id
        if m.focused then
            focused_monitor = m.name
        end
    end
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
    
    local item_size = 24
    local item_spacing = 30
    local width = (math.max(count, 1) - 1) * item_spacing + item_size
    return math.ceil(width), item_size
end

function view(canvas, monitor)
    local monitor_id = monitor:id()
    local active_id = active_workspaces[monitor_id] or -1
    local is_monitor_focused = focused_monitor == monitor_id
    
    local item_size = 24
    local item_spacing = 30
    local x_offset = 0
    
    local inactive_color = "#7aa2f7"
    local active_text_color = "#ffffff"
    
    for _, ws in ipairs(workspaces) do
        if ws.monitor == monitor_id then
            local label = tostring(ws.id)
            local is_visible = ws.id == active_id
            
            if is_visible then
                local bg = is_monitor_focused and active_bg or focused_bg
                canvas:draw_rect(x_offset, 0, item_size, item_size, bg, border_radius)
                
                local lw, lh = canvas:measure_text(label, font_family, font_size)
                canvas:draw_text(label, font_family, font_size, active_text_color, x_offset + (item_size - lw) / 2, (item_size - lh) / 2)
            else
                local lw, lh = canvas:measure_text(label, font_family, font_size)
                canvas:draw_text(label, font_family, font_size, inactive_color, x_offset + (item_size - lw) / 2, (item_size - lh) / 2)
            end
            x_offset = x_offset + item_spacing
        end
    end
end
