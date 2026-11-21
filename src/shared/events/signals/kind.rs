//! The kinds of signal a module can subscribe to.

/// A state source a module can subscribe to in its `metadata()`; the module re-renders only when
/// one of its sources changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalKind {
    /// The local clock.
    Time,
    /// Hyprland workspaces, monitors and focus.
    Hyprland,
    /// Properties and signals from the `DBus` subscriptions scripts declare.
    DBus,
    /// Status notifier (tray) items.
    Systray,
    /// System metrics such as CPU, memory and disks.
    Metrics,
    /// Media players over MPRIS.
    Mpris,
}
