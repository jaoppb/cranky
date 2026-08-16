local active_bg = "#3b4261"
local focused_bg = "#565f89"
local border_radius = 0
local inactive_color = "#7aa2f7"
local active_text_color = "#ffffff"

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
    if config.border_radius then border_radius = config.border_radius end
    if config.inactive_color then inactive_color = config.inactive_color end
    if config.active_text_color then active_text_color = config.active_text_color end
end

function subscriptions()
    return { "hyprland" }
end

function refresh()
    if not hyprland then return end
    
    workspaces = hyprland.workspaces
    focused_monitor = hyprland.focused_monitor or ""
    active_workspaces = {}
    for _, m in ipairs(hyprland.monitors) do
        active_workspaces[m.name] = {
            active = m.active_workspace_id,
            special = m.special_workspace_id
        }
        
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
                name = tostring(m.active_workspace_id),
                monitor = m.name
            })
        end

        if type(m.special_workspace_id) == "number" and m.special_workspace_id ~= 0 then
            local found_sp = false
            for _, ws in ipairs(workspaces) do
                if ws.id == m.special_workspace_id then
                    found_sp = true
                    break
                end
            end
            if not found_sp then
                table.insert(workspaces, {
                    id = m.special_workspace_id,
                    name = "special",
                    monitor = m.name
                })
            end
        end
    end
    
    table.sort(workspaces, function(a, b)
        if a.id > 0 and b.id > 0 then return a.id < b.id end
        if a.id < 0 and b.id < 0 then return a.id > b.id end
        return a.id > b.id
    end)
end

function render(monitor)
    local monitor_id = monitor:id()
    local active_ids = active_workspaces[monitor_id] or { active = -1 }
    
    local children = {}
    
    for _, ws in ipairs(workspaces) do
        if ws.monitor == monitor_id then
            local is_special = ws.name:match("^special:") or (ws.id < 0)
            local label = ws.name:match("^special:(.*)") or ws.name
            if label == "" or label == tostring(ws.id) then
                if is_special then label = "special" end
            end
            
            local active_ws = (type(active_ids.special) == "number" and active_ids.special ~= 0) and active_ids.special or active_ids.active
            local is_visible = (ws.id == active_ws)
            
            local color = is_visible and active_text_color or inactive_color
            
            local exec_cmd
            if is_special then
                exec_cmd = "hyprctl dispatch togglespecialworkspace " .. label
            else
                exec_cmd = "hyprctl dispatch workspace " .. ws.id
            end

            local text_node = {
                type = "text",
                text = label,
                color = color
            }
            
            local bg = "#00000000"
            if is_visible then
                if focused_monitor == monitor_id then
                    bg = focused_bg
                else
                    bg = active_bg
                end
            end
            
            table.insert(children, {
                type = "flex",
                style = { 
                    justify = "center", 
                    align_items = "center", 
                    padding = { top = 4, bottom = 4, left = 8, right = 8 } 
                },
                background = bg,
                radius = border_radius,
                on_click = { Exec = exec_cmd },
                tooltip = {
                    type = "flex",
                    background = "#1e1e2eff",
                    radius = 4,
                    style = { padding = { top = 4, bottom = 4, left = 8, right = 8 } },
                    children = {
                        { type = "text", text = "Switch to workspace " .. label, color = "#ffffff" }
                    }
                },
                children = { text_node }
            })
        end
    end
    
    return {
        type = "flex",
        style = { gap = 6, align_items = "center" },
        children = children
    }
end
