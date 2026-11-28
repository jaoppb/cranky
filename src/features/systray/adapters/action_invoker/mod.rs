mod failures;
mod item_call;
mod plan;

use crate::features::systray::domain::{ItemIsMenu, SystrayActionCall, SystrayItem};
use crate::shared::primitives::geometry::Position;
use failures::{ContextMenuFailures, EarlierFailures};
use item_call::{ItemCall, ItemMethods};
use plan::{ActionPlan, plan};
use tracing::{debug, error, warn};

/// Logs the item call about to be made, out of line to keep its caller under `cognitive_complexity`.
fn log_calling(call: ItemCall) {
    debug!("trigger_action: Calling {call} on D-Bus");
}

/// Logs the primary call about to be made on an item, out of line to keep its caller under `cognitive_complexity`.
fn log_primary_calling(item_is_menu: ItemIsMenu, call: ItemCall) {
    debug!(
        "trigger_action: item_is_menu={}, calling {call} on D-Bus",
        bool::from(item_is_menu)
    );
}

/// Logs that an item call succeeded, out of line to keep its caller under `cognitive_complexity`.
fn log_succeeded(call: ItemCall) {
    debug!("trigger_action: {call} succeeded");
}

/// Logs that an item call failed, out of line to keep its caller under `cognitive_complexity`.
fn log_failed(call: ItemCall, error: &zbus::Error) {
    error!("trigger_action: {call} failed: {error}");
}

/// A call the item rejected, and the error it answered with.
struct FailedCall<'error> {
    /// The rejected call.
    call: ItemCall,
    /// What the item answered.
    error: &'error zbus::Error,
}

/// Logs the fallback call tried after one failed, out of line to keep its caller under `cognitive_complexity`.
fn log_falling_back(failed: &FailedCall<'_>, fallback: ItemCall) {
    let FailedCall { call, error } = failed;
    warn!("trigger_action: {call} failed: {error}, attempting {fallback}");
}

/// Logs that every way of opening the context menu failed, out of line to keep its caller under `cognitive_complexity`.
fn log_context_menu_failed(position: Position, failures: &ContextMenuFailures) {
    let call = ItemCall::ContextMenu(position);
    error!("trigger_action: {call} failed: {failures}");
}

/// Logs an action the invoker does not recognize, out of line to keep its caller under `cognitive_complexity`.
fn log_unrecognized(action: &str) {
    warn!("trigger_action: Unrecognized action '{action}'");
}

/// Logs how `call` ended: a debug line when it succeeded, an error line when it failed.
fn report(call: ItemCall, result: &zbus::Result<()>) {
    match result {
        Ok(()) => log_succeeded(call),
        Err(error) => log_failed(call, error),
    }
}

/// Sends one call with no fallback, logging it before and after.
async fn call_reported(item: &impl ItemMethods, call: ItemCall) {
    log_calling(call);
    report(call, &item.call(call).await);
}

/// Sends `call`, then `fallback` when the item rejects it, logging the outcome of each.
async fn call_with_fallback(item: &impl ItemMethods, call: ItemCall, fallback: ItemCall) {
    match item.call(call).await {
        Ok(()) => log_succeeded(call),
        Err(error) => {
            log_falling_back(
                &FailedCall {
                    call,
                    error: &error,
                },
                fallback,
            );
            report(fallback, &item.call(fallback).await);
        }
    }
}

/// A primary click: `ContextMenu` for a menu-only item, otherwise `Activate` falling back to
/// `SecondaryActivate`.
async fn invoke_primary(item: &impl ItemMethods, item_is_menu: ItemIsMenu, position: Position) {
    if item_is_menu == ItemIsMenu::MenuOnly {
        let call = ItemCall::ContextMenu(position);
        log_primary_calling(item_is_menu, call);
        report(call, &item.call(call).await);
    } else {
        log_primary_calling(item_is_menu, ItemCall::Activate(position));
        call_with_fallback(
            item,
            ItemCall::Activate(position),
            ItemCall::SecondaryActivate(position),
        )
        .await;
    }
}

/// `ContextMenu`, falling back to `SecondaryActivate` and then to `Activate`.
async fn invoke_context_menu(item: &impl ItemMethods, position: Position) {
    let call = ItemCall::ContextMenu(position);
    log_calling(call);
    match item.call(call).await {
        Ok(()) => log_succeeded(call),
        Err(error) => context_menu_fallbacks(item, position, error).await,
    }
}

/// The fallbacks of a failed `ContextMenu`: `SecondaryActivate`, then `Activate`.
async fn context_menu_fallbacks(
    item: &impl ItemMethods,
    position: Position,
    context_menu: zbus::Error,
) {
    let secondary_call = ItemCall::SecondaryActivate(position);
    let failed = FailedCall {
        call: ItemCall::ContextMenu(position),
        error: &context_menu,
    };
    log_falling_back(&failed, secondary_call);
    match item.call(secondary_call).await {
        Ok(()) => log_succeeded(secondary_call),
        Err(secondary) => {
            let earlier = EarlierFailures::new(context_menu, secondary);
            activate_last(item, position, earlier).await;
        }
    }
}

/// The last fallback of a `ContextMenu`, `Activate`; only when it fails too is the whole chain
/// logged as an error.
async fn activate_last(item: &impl ItemMethods, position: Position, earlier: EarlierFailures) {
    let call = ItemCall::Activate(position);
    match item.call(call).await {
        Ok(()) => log_succeeded(call),
        Err(activate) => log_context_menu_failed(position, &earlier.and_activate(activate)),
    }
}

/// Performs `call` on the item, at its position or (0, 0) without one. Every outcome is logged,
/// never returned: a failed D-Bus call is the item's problem, not the tray's.
pub(super) async fn invoke_action(
    proxy: &impl ItemMethods,
    item: &SystrayItem,
    call: &SystrayActionCall,
) {
    let position = call.position().unwrap_or_else(|| Position::new(0, 0));
    match plan(call.name().as_str(), position) {
        ActionPlan::Primary => invoke_primary(proxy, item.item_is_menu(), position).await,
        ActionPlan::ContextMenu => invoke_context_menu(proxy, position).await,
        ActionPlan::Single(item_call) => call_reported(proxy, item_call).await,
        ActionPlan::Unrecognized(other) => log_unrecognized(other),
    }
}

#[cfg(test)]
mod tests {
    use super::{ItemCall, ItemMethods, invoke_action};
    use crate::features::module_runtime::test_support::capture_logs;
    use crate::features::systray::domain::{
        CreateSystrayItemCommand, Destination, ItemIsMenu, ObjectPath, SystrayActionCall,
        SystrayActionName, SystrayAddress, SystrayId, SystrayItem, SystrayItemParams, Title,
    };
    use crate::shared::primitives::geometry::Position;
    use std::sync::Mutex;

    /// The target every line of the invoker is logged under.
    const TARGET: &str = "cranky::features::systray::adapters::action_invoker";

    /// An item that records every call it receives and rejects the methods it is told to.
    struct FakeItem {
        rejects: Vec<&'static str>,
        calls: Mutex<Vec<String>>,
    }

    impl ItemMethods for FakeItem {
        fn call(&self, call: ItemCall) -> impl Future<Output = zbus::Result<()>> + Send {
            self.calls.lock().unwrap().push(call.to_string());
            let rejected = self.rejects.contains(&call.method());
            let rejection =
                rejected.then(|| zbus::Error::Failure(format!("{} rejected", call.method())));
            std::future::ready(rejection.map_or(Ok(()), Err))
        }
    }

    /// An action at a position, or at none.
    struct Trigger(SystrayActionName, Option<Position>);

    /// The calls an item with `item_is_menu`, rejecting the `rejects` methods, receives when
    /// `trigger` fires.
    fn calls(trigger: Trigger, item_is_menu: ItemIsMenu, rejects: &[&'static str]) -> Vec<String> {
        calls_and_logs(trigger, item_is_menu, rejects).0
    }

    /// The calls, as `calls` returns them, and the lines logged while they were made.
    fn calls_and_logs(
        trigger: Trigger,
        item_is_menu: ItemIsMenu,
        rejects: &[&'static str],
    ) -> (Vec<String>, String) {
        let fake = FakeItem {
            rejects: rejects.to_vec(),
            calls: Mutex::new(Vec::new()),
        };
        let item = SystrayItem::new(CreateSystrayItemCommand::new(
            SystrayItemParams::new(
                SystrayId::new("item"),
                SystrayAddress::new(Destination::new(":1.1"), ObjectPath::new("/item")),
                Title::new("Item"),
            )
            .with_item_is_menu(item_is_menu),
        ));
        let Trigger(name, position) = trigger;
        let call = SystrayActionCall::new(name, position);
        let logs = capture_logs(|| {
            crate::test_utils::block_on(invoke_action(&fake, &item, &call));
        });
        (fake.calls.into_inner().unwrap(), logs)
    }

    /// The calls a plain (activatable) item receives for `name` at (3, 4).
    fn plain(name: SystrayActionName, rejects: &[&'static str]) -> Vec<String> {
        calls(
            Trigger(name, Some(Position::new(3, 4))),
            ItemIsMenu::Activatable,
            rejects,
        )
    }

    #[test]
    fn test_invoke_action_sends_each_simple_action_once() {
        let expected = [
            (SystrayActionName::Activate, "Activate(3, 4)"),
            (
                SystrayActionName::SecondaryActivate,
                "SecondaryActivate(3, 4)",
            ),
            (SystrayActionName::ScrollUp, "Scroll(-1, 'vertical')"),
            (SystrayActionName::ScrollDown, "Scroll(1, 'vertical')"),
            (SystrayActionName::ScrollLeft, "Scroll(-1, 'horizontal')"),
            (SystrayActionName::ScrollRight, "Scroll(1, 'horizontal')"),
        ];
        for (name, call) in expected {
            // A rejected simple action has no fallback.
            assert_eq!(
                plain(name, &["Activate", "SecondaryActivate", "Scroll"]),
                [call]
            );
        }
    }

    #[test]
    fn test_invoke_action_without_position_sends_the_origin() {
        let trigger = Trigger(SystrayActionName::Activate, None);
        assert_eq!(
            calls(trigger, ItemIsMenu::Activatable, &[]),
            ["Activate(0, 0)"]
        );
    }

    #[test]
    fn test_invoke_action_primary_opens_the_menu_of_a_menu_only_item() {
        let trigger = || Trigger(SystrayActionName::Primary, Some(Position::new(3, 4)));
        assert_eq!(
            calls(trigger(), ItemIsMenu::MenuOnly, &[]),
            ["ContextMenu(3, 4)"]
        );
        // No fallback when the menu fails.
        let rejected = calls(trigger(), ItemIsMenu::MenuOnly, &["ContextMenu"]);
        assert_eq!(rejected, ["ContextMenu(3, 4)"]);
    }

    #[test]
    fn test_invoke_action_primary_falls_back_to_secondary_activate() {
        let primary = || SystrayActionName::Primary;
        assert_eq!(plain(primary(), &[]), ["Activate(3, 4)"]);
        let fallback = ["Activate(3, 4)", "SecondaryActivate(3, 4)"];
        assert_eq!(plain(primary(), &["Activate"]), fallback);
        assert_eq!(
            plain(primary(), &["Activate", "SecondaryActivate"]),
            fallback
        );
    }

    #[test]
    fn test_invoke_action_context_menu_falls_back_twice() {
        let menu = || SystrayActionName::ContextMenu;
        assert_eq!(plain(menu(), &[]), ["ContextMenu(3, 4)"]);
        let secondary = ["ContextMenu(3, 4)", "SecondaryActivate(3, 4)"];
        assert_eq!(plain(menu(), &["ContextMenu"]), secondary);
        let all = [
            "ContextMenu(3, 4)",
            "SecondaryActivate(3, 4)",
            "Activate(3, 4)",
        ];
        assert_eq!(plain(menu(), &["ContextMenu", "SecondaryActivate"]), all);
        let rejects = ["ContextMenu", "SecondaryActivate", "Activate"];
        assert_eq!(plain(menu(), &rejects), all);
    }

    #[test]
    fn test_invoke_action_ignores_an_unrecognized_action() {
        let name = SystrayActionName::Other("Bogus".to_string());
        assert_eq!(plain(name, &[]), Vec::<String>::new());
    }

    #[test]
    fn test_invoke_action_logs_a_context_menu_whose_fallbacks_all_failed() {
        let trigger = Trigger(SystrayActionName::ContextMenu, Some(Position::new(3, 4)));
        let rejects = ["ContextMenu", "SecondaryActivate", "Activate"];
        let (_, logs) = calls_and_logs(trigger, ItemIsMenu::Activatable, &rejects);
        assert_eq!(
            logs,
            format!(
                "DEBUG {TARGET}: trigger_action: Calling ContextMenu(3, 4) on D-Bus\n \
                 WARN {TARGET}: trigger_action: ContextMenu(3, 4) failed: ContextMenu rejected, \
                 attempting SecondaryActivate(3, 4)\n\
                 ERROR {TARGET}: trigger_action: ContextMenu(3, 4) failed: ContextMenu rejected \
                 (fallback SecondaryActivate: SecondaryActivate rejected, Activate: Activate \
                 rejected)\n"
            )
        );
    }

    #[test]
    fn test_invoke_action_logs_a_primary_falling_back_to_secondary_activate() {
        let trigger = Trigger(SystrayActionName::Primary, Some(Position::new(3, 4)));
        let (_, logs) = calls_and_logs(trigger, ItemIsMenu::Activatable, &["Activate"]);
        assert_eq!(
            logs,
            format!(
                "DEBUG {TARGET}: trigger_action: item_is_menu=false, calling Activate(3, 4) on \
                 D-Bus\n \
                 WARN {TARGET}: trigger_action: Activate(3, 4) failed: Activate rejected, \
                 attempting SecondaryActivate(3, 4)\n\
                 DEBUG {TARGET}: trigger_action: SecondaryActivate(3, 4) succeeded\n"
            )
        );
    }
}
