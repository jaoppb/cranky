use crate::domain::signals::HyprlandState;
use crate::domain::dbus::DBusValue;

#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    PointerEnter,
    PointerLeave,
    Click { button: u32, x: f64, y: f64 },
    Scroll { axis: u32, amount: f64 },
    Time(chrono::DateTime<chrono::Local>),
    HyprlandState(HyprlandState),
    DBusProperty(String, DBusValue),
    AppletsState(crate::domain::applets::AppletsState),
    MetricsState(crate::domain::metrics::MetricsState),
}
