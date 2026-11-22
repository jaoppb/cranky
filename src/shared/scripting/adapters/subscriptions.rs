//! What both engines read the same way from a script's `metadata()`: the signals named by a
//! string, the bus a `DBus` subscription names, and the style sheet a module falls back to.
use crate::features::styling::domain::StyleSheetName;
use crate::shared::dbus::domain::BusType;
use crate::shared::events::signals::SignalKind;

/// The signal a subscription string names; `DBus` is subscribed with a table, never a name.
pub(super) fn signal_kind_named(name: &str) -> Option<SignalKind> {
    match name {
        "time" => Some(SignalKind::Time),
        "hyprland" => Some(SignalKind::Hyprland),
        "systray" => Some(SignalKind::Systray),
        "metrics" => Some(SignalKind::Metrics),
        "mpris" => Some(SignalKind::Mpris),
        _ => None,
    }
}

/// The bus a `DBus` subscription names: the system bus for `"system"`, else the session bus.
pub(super) fn bus_type_named(name: Option<&str>) -> BusType {
    if name == Some("system") {
        BusType::System
    } else {
        BusType::Session
    }
}

/// `styles`, or the style sheet named after the module when there are none.
pub(super) fn with_default_style(
    mut styles: Vec<StyleSheetName>,
    module_name: &str,
) -> Vec<StyleSheetName> {
    if styles.is_empty()
        && let Ok(default_sheet) = StyleSheetName::new(module_name)
    {
        styles.push(default_sheet);
    }
    styles
}
