use crate::features::systray::domain::{SystrayActionName, SystrayItem};
use crate::shared::primitives::geometry::Position;
use tracing::{debug, error, warn};
use zbus::Proxy;

#[allow(clippy::too_many_lines)]
pub async fn invoke_action(
    proxy: &Proxy<'_>,
    item: &SystrayItem,
    action: &SystrayActionName,
    pos: Option<Position>,
) {
    let pos_x = pos.map_or(0, |p| p.x());
    let pos_y = pos.map_or(0, |p| p.y());

    match action.as_str() {
        "Primary" => {
            if item.item_is_menu().value() {
                debug!(
                    "trigger_action: item_is_menu=true, calling ContextMenu({pos_x}, {pos_y}) on D-Bus"
                );
                match proxy.call_method("ContextMenu", &(pos_x, pos_y)).await {
                    Ok(_) => {
                        debug!("trigger_action: ContextMenu({pos_x}, {pos_y}) succeeded");
                    }
                    Err(e) => {
                        error!("trigger_action: ContextMenu({pos_x}, {pos_y}) failed: {e}");
                    }
                }
            } else {
                debug!(
                    "trigger_action: item_is_menu=false, calling Activate({pos_x}, {pos_y}) on D-Bus"
                );
                match proxy.call_method("Activate", &(pos_x, pos_y)).await {
                    Ok(_) => debug!("trigger_action: Activate({pos_x}, {pos_y}) succeeded"),
                    Err(e) => {
                        warn!(
                            "trigger_action: Activate({pos_x}, {pos_y}) failed: {e}, attempting SecondaryActivate({pos_x}, {pos_y})"
                        );
                        match proxy
                            .call_method("SecondaryActivate", &(pos_x, pos_y))
                            .await
                        {
                            Ok(_) => debug!(
                                "trigger_action: SecondaryActivate({pos_x}, {pos_y}) succeeded"
                            ),
                            Err(e2) => error!(
                                "trigger_action: SecondaryActivate({pos_x}, {pos_y}) failed: {e2}"
                            ),
                        }
                    }
                }
            }
        }
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
                Ok(_) => {
                    debug!("trigger_action: SecondaryActivate({pos_x}, {pos_y}) succeeded");
                }
                Err(e) => error!(
                    "trigger_action: SecondaryActivate({pos_x}, {pos_y}) failed: {e}"
                ),
            }
        }
        "ContextMenu" => {
            debug!("trigger_action: Calling ContextMenu({pos_x}, {pos_y}) on D-Bus");
            match proxy.call_method("ContextMenu", &(pos_x, pos_y)).await {
                Ok(_) => debug!("trigger_action: ContextMenu({pos_x}, {pos_y}) succeeded"),
                Err(e) => {
                    warn!(
                        "trigger_action: ContextMenu({pos_x}, {pos_y}) failed: {e}, attempting SecondaryActivate({pos_x}, {pos_y})"
                    );
                    match proxy
                        .call_method("SecondaryActivate", &(pos_x, pos_y))
                        .await
                    {
                        Ok(_) => debug!(
                            "trigger_action: SecondaryActivate({pos_x}, {pos_y}) succeeded"
                        ),
                        Err(e2) => {
                            match proxy.call_method("Activate", &(pos_x, pos_y)).await {
                                Ok(_) => debug!(
                                    "trigger_action: Activate({pos_x}, {pos_y}) succeeded"
                                ),
                                Err(e3) => error!(
                                    "trigger_action: ContextMenu({pos_x}, {pos_y}) failed: {e} (fallback SecondaryActivate: {e2}, Activate: {e3})"
                                ),
                            }
                        }
                    }
                }
            }
        }
        "ScrollUp" => {
            debug!("trigger_action: Calling Scroll(-1, 'vertical') on D-Bus");
            match proxy.call_method("Scroll", &(-1, "vertical")).await {
                Ok(_) => debug!("trigger_action: Scroll(-1, 'vertical') succeeded"),
                Err(e) => error!("trigger_action: Scroll(-1, 'vertical') failed: {e}"),
            }
        }
        "ScrollDown" => {
            debug!("trigger_action: Calling Scroll(1, 'vertical') on D-Bus");
            match proxy.call_method("Scroll", &(1, "vertical")).await {
                Ok(_) => debug!("trigger_action: Scroll(1, 'vertical') succeeded"),
                Err(e) => error!("trigger_action: Scroll(1, 'vertical') failed: {e}"),
            }
        }
        "ScrollLeft" => {
            debug!("trigger_action: Calling Scroll(-1, 'horizontal') on D-Bus");
            match proxy.call_method("Scroll", &(-1, "horizontal")).await {
                Ok(_) => debug!("trigger_action: Scroll(-1, 'horizontal') succeeded"),
                Err(e) => error!("trigger_action: Scroll(-1, 'horizontal') failed: {e}"),
            }
        }
        "ScrollRight" => {
            debug!("trigger_action: Calling Scroll(1, 'horizontal') on D-Bus");
            match proxy.call_method("Scroll", &(1, "horizontal")).await {
                Ok(_) => debug!("trigger_action: Scroll(1, 'horizontal') succeeded"),
                Err(e) => error!("trigger_action: Scroll(1, 'horizontal') failed: {e}"),
            }
        }
        other => {
            warn!("trigger_action: Unrecognized action '{other}'");
        }
    }
}
