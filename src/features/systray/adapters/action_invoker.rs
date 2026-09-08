use crate::features::systray::domain::{SystrayActionName, SystrayItem};
use crate::shared::primitives::geometry::Position;
use tracing::{debug, error, warn};
use zbus::Proxy;

async fn invoke_primary(proxy: &Proxy<'_>, is_menu: bool, pos_x: i32, pos_y: i32) {
    if is_menu {
        debug!(
            "trigger_action: item_is_menu=true, calling ContextMenu({pos_x}, {pos_y}) on D-Bus"
        );
        match proxy.call_method("ContextMenu", &(pos_x, pos_y)).await {
            Ok(_) => debug!("trigger_action: ContextMenu({pos_x}, {pos_y}) succeeded"),
            Err(e) => error!("trigger_action: ContextMenu({pos_x}, {pos_y}) failed: {e}"),
        }
    } else {
        debug!(
            "trigger_action: item_is_menu=false, calling Activate({pos_x}, {pos_y}) on D-Bus"
        );
        if let Err(e) = proxy.call_method("Activate", &(pos_x, pos_y)).await {
            warn!(
                "trigger_action: Activate({pos_x}, {pos_y}) failed: {e}, attempting SecondaryActivate({pos_x}, {pos_y})"
            );
            match proxy
                .call_method("SecondaryActivate", &(pos_x, pos_y))
                .await
            {
                Ok(_) => debug!("trigger_action: SecondaryActivate({pos_x}, {pos_y}) succeeded"),
                Err(e2) => {
                    error!("trigger_action: SecondaryActivate({pos_x}, {pos_y}) failed: {e2}");
                }
            }
        } else {
            debug!("trigger_action: Activate({pos_x}, {pos_y}) succeeded");
        }
    }
}

async fn invoke_context_menu(proxy: &Proxy<'_>, pos_x: i32, pos_y: i32) {
    debug!("trigger_action: Calling ContextMenu({pos_x}, {pos_y}) on D-Bus");
    if let Err(e) = proxy.call_method("ContextMenu", &(pos_x, pos_y)).await {
        warn!(
            "trigger_action: ContextMenu({pos_x}, {pos_y}) failed: {e}, attempting SecondaryActivate({pos_x}, {pos_y})"
        );
        match proxy
            .call_method("SecondaryActivate", &(pos_x, pos_y))
            .await
        {
            Ok(_) => debug!("trigger_action: SecondaryActivate({pos_x}, {pos_y}) succeeded"),
            Err(e2) => match proxy.call_method("Activate", &(pos_x, pos_y)).await {
                Ok(_) => debug!("trigger_action: Activate({pos_x}, {pos_y}) succeeded"),
                Err(e3) => error!(
                    "trigger_action: ContextMenu({pos_x}, {pos_y}) failed: {e} (fallback SecondaryActivate: {e2}, Activate: {e3})"
                ),
            },
        }
    } else {
        debug!("trigger_action: ContextMenu({pos_x}, {pos_y}) succeeded");
    }
}

async fn invoke_scroll(proxy: &Proxy<'_>, delta: i32, orientation: &str) {
    debug!("trigger_action: Calling Scroll({delta}, '{orientation}') on D-Bus");
    match proxy.call_method("Scroll", &(delta, orientation)).await {
        Ok(_) => debug!("trigger_action: Scroll({delta}, '{orientation}') succeeded"),
        Err(e) => error!("trigger_action: Scroll({delta}, '{orientation}') failed: {e}"),
    }
}

pub async fn invoke_action(
    proxy: &Proxy<'_>,
    item: &SystrayItem,
    action: &SystrayActionName,
    pos: Option<Position>,
) {
    let pos_x = pos.map_or(0, |p| p.x());
    let pos_y = pos.map_or(0, |p| p.y());

    match action.as_str() {
        "Primary" => invoke_primary(proxy, item.item_is_menu().value(), pos_x, pos_y).await,
        "Activate" => {
            debug!("trigger_action: Calling Activate({pos_x}, {pos_y}) on D-Bus");
            match proxy.call_method("Activate", &(pos_x, pos_y)).await {
                Ok(_) => debug!("trigger_action: Activate({pos_x}, {pos_y}) succeeded"),
                Err(e) => error!("trigger_action: Activate({pos_x}, {pos_y}) failed: {e}"),
            }
        }
        "SecondaryActivate" => {
            debug!("trigger_action: Calling SecondaryActivate({pos_x}, {pos_y}) on D-Bus");
            match proxy
                .call_method("SecondaryActivate", &(pos_x, pos_y))
                .await
            {
                Ok(_) => debug!("trigger_action: SecondaryActivate({pos_x}, {pos_y}) succeeded"),
                Err(e) => {
                    error!("trigger_action: SecondaryActivate({pos_x}, {pos_y}) failed: {e}");
                }
            }
        }
        "ContextMenu" => invoke_context_menu(proxy, pos_x, pos_y).await,
        "ScrollUp" => invoke_scroll(proxy, -1, "vertical").await,
        "ScrollDown" => invoke_scroll(proxy, 1, "vertical").await,
        "ScrollLeft" => invoke_scroll(proxy, -1, "horizontal").await,
        "ScrollRight" => invoke_scroll(proxy, 1, "horizontal").await,
        other => warn!("trigger_action: Unrecognized action '{other}'"),
    }
}
