//! The calls an action makes on an item, and the item side that receives them.
use crate::shared::primitives::geometry::Position;
use std::fmt;
use zbus::Proxy;

/// One method call on an item's `org.kde.StatusNotifierItem` interface, with its arguments.
/// Displays as the logs name it: `Activate(3, 4)`, `Scroll(-1, 'vertical')`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::features::systray::adapters) enum ItemCall {
    /// `Activate(x, y)`.
    Activate(Position),
    /// `SecondaryActivate(x, y)`.
    SecondaryActivate(Position),
    /// `ContextMenu(x, y)`.
    ContextMenu(Position),
    /// `Scroll(delta, orientation)`.
    Scroll(i32, &'static str),
}

impl fmt::Display for ItemCall {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Activate(position)
            | Self::SecondaryActivate(position)
            | Self::ContextMenu(position) => {
                write!(
                    formatter,
                    "{}({}, {})",
                    self.method(),
                    position.x(),
                    position.y()
                )
            }
            Self::Scroll(delta, orientation) => {
                write!(formatter, "Scroll({delta}, '{orientation}')")
            }
        }
    }
}

impl ItemCall {
    /// The D-Bus method name.
    pub(super) const fn method(&self) -> &'static str {
        match self {
            Self::Activate(_) => "Activate",
            Self::SecondaryActivate(_) => "SecondaryActivate",
            Self::ContextMenu(_) => "ContextMenu",
            Self::Scroll(..) => "Scroll",
        }
    }
}

/// The item side of an action: sends one [`ItemCall`] and reports whether the item accepted it.
/// The D-Bus proxy implements it; the tests use a fake item that records the calls.
pub(in crate::features::systray::adapters) trait ItemMethods:
    Sync
{
    /// Sends `call` to the item.
    fn call(&self, call: ItemCall) -> impl Future<Output = zbus::Result<()>> + Send;
}

impl ItemMethods for Proxy<'_> {
    async fn call(&self, call: ItemCall) -> zbus::Result<()> {
        match call {
            ItemCall::Activate(position)
            | ItemCall::SecondaryActivate(position)
            | ItemCall::ContextMenu(position) => {
                self.call_method(call.method(), &(position.x(), position.y()))
                    .await
            }
            ItemCall::Scroll(delta, orientation) => {
                self.call_method(call.method(), &(delta, orientation)).await
            }
        }
        .map(|_| ())
    }
}
