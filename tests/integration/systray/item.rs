//! A fake `org.kde.StatusNotifierItem` on the private bus.
//!
//! Plain Rust types only, so the `zbus` bump of #30 has nothing to migrate.

use crate::support::{Context, Outcome};
use tokio::sync::mpsc;
use zbus::object_server::{InterfaceRef, SignalEmitter};

pub(crate) const PATH: &str = "/StatusNotifierItem";

type Pixmaps = Vec<(i32, i32, Vec<u8>)>;

/// A method the tray called on the item, with its arguments.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Call {
    Activate(i32, i32),
    ContextMenu(i32, i32),
    Scroll(i32, String),
}

/// The `ItemIsMenu` property: whether the item only offers its context
/// menu. Reported on the bus as the `bool` the property carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ItemIsMenu {
    /// The item only offers its menu (`true`).
    MenuOnly,
    /// The item can be activated (`false`).
    Activatable,
}

struct Item {
    title: String,
    status: String,
    category: &'static str,
    id: &'static str,
    is_menu: ItemIsMenu,
    tool_tip: (String, Pixmaps, String, String),
    calls: mpsc::UnboundedSender<Call>,
}

impl Item {
    fn record(&self, call: Call) {
        // The receiver only goes away when the case is over.
        let _ = self.calls.send(call);
    }
}

#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl Item {
    #[zbus(property)]
    fn title(&self) -> String {
        self.title.clone()
    }

    #[zbus(property)]
    fn status(&self) -> String {
        self.status.clone()
    }

    #[zbus(property)]
    fn category(&self) -> String {
        self.category.to_string()
    }

    #[zbus(property)]
    fn id(&self) -> String {
        self.id.to_string()
    }

    #[zbus(property)]
    const fn item_is_menu(&self) -> bool {
        matches!(self.is_menu, ItemIsMenu::MenuOnly)
    }

    #[zbus(property)]
    fn tool_tip(&self) -> (String, Pixmaps, String, String) {
        self.tool_tip.clone()
    }

    fn activate(&self, x: i32, y: i32) {
        self.record(Call::Activate(x, y));
    }

    fn context_menu(&self, x: i32, y: i32) {
        self.record(Call::ContextMenu(x, y));
    }

    fn scroll(&self, delta: i32, orientation: String) {
        self.record(Call::Scroll(delta, orientation));
    }

    #[zbus(signal)]
    async fn new_title(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_status(emitter: &SignalEmitter<'_>, status: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_menu(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// The fake item's own connection to the private bus.
pub(crate) struct ItemService {
    connection: zbus::Connection,
    calls: mpsc::UnboundedReceiver<Call>,
}

impl ItemService {
    pub(crate) async fn start() -> Outcome<Self> {
        let (sender, calls) = mpsc::unbounded_channel();
        let item = Item {
            title: "Fake Title".to_string(),
            status: "Active".to_string(),
            category: "Communications",
            id: "fake-item",
            is_menu: ItemIsMenu::Activatable,
            tool_tip: (
                String::new(),
                Pixmaps::new(),
                "<b>Tip</b>".to_string(),
                "a &amp; b".to_string(),
            ),
            calls: sender,
        };
        let connection = zbus::connection::Builder::session()
            .serve_at(PATH, item)
            .build()
            .await
            .context("item: connect")?;
        Ok(Self { connection, calls })
    }

    /// The id the tray gives an item that registers by object path.
    pub(crate) fn tray_id(&self) -> Outcome<String> {
        let name = self
            .connection
            .unique_name()
            .context("the item connection has no unique name")?;
        Ok(format!("{name}{PATH}"))
    }

    /// Registers with the watcher by object path, as real items do.
    pub(crate) async fn register(&self) -> Outcome {
        self.connection
            .call_method(
                Some("org.kde.StatusNotifierWatcher"),
                "/StatusNotifierWatcher",
                Some("org.kde.StatusNotifierWatcher"),
                "RegisterStatusNotifierItem",
                &(PATH,),
            )
            .await
            .map(|_| ())
            .context("item: RegisterStatusNotifierItem")
    }

    async fn interface(&self) -> Outcome<InterfaceRef<Item>> {
        self.connection
            .object_server()
            .interface::<_, Item>(PATH)
            .await
            .context("item: interface")
    }

    /// Sets `ItemIsMenu`; done before the item registers, the tray fetches it
    /// as an item that only offers its menu.
    pub(crate) async fn mark_as_menu(&self) -> Outcome {
        self.interface().await?.get_mut().await.is_menu = ItemIsMenu::MenuOnly;
        Ok(())
    }

    /// Changes `Title` and announces it with `NewTitle`.
    pub(crate) async fn retitle(&self, title: &str) -> Outcome {
        let interface = self.interface().await?;
        interface.get_mut().await.title = title.to_string();
        Item::new_title(interface.signal_emitter())
            .await
            .context("item: emit NewTitle")
    }

    /// Changes `Status` and announces it with `NewStatus`.
    pub(crate) async fn restatus(&self, status: &str) -> Outcome {
        let interface = self.interface().await?;
        interface.get_mut().await.status = status.to_string();
        Item::new_status(interface.signal_emitter(), status)
            .await
            .context("item: emit NewStatus")
    }

    /// Sets `ItemIsMenu` on a registered item and announces it with `NewMenu`.
    pub(crate) async fn become_menu(&self) -> Outcome {
        let interface = self.interface().await?;
        interface.get_mut().await.is_menu = ItemIsMenu::MenuOnly;
        Item::new_menu(interface.signal_emitter())
            .await
            .context("item: emit NewMenu")
    }

    /// The next method the tray called on the item.
    pub(crate) async fn next_call(&mut self) -> Option<Call> {
        self.calls.recv().await
    }

    /// Leaves the bus, as an application that quit.
    pub(crate) async fn disconnect(self) -> Outcome {
        self.connection.close().await.context("item: disconnect")
    }
}
