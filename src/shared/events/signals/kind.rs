#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalKind {
    Time,
    Hyprland,
    DBus,
    Systray,
    Metrics,
    Mpris,
}
