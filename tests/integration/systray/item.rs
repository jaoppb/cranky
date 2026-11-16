//! A fake `org.kde.StatusNotifierItem` on the private bus.
//!
//! Plain Rust types only, so the `zbus` bump of #30 has nothing to migrate.

use crate::support::{Context, Outcome};
use std::sync::{Arc, Mutex, PoisonError};
use tokio::sync::mpsc;
use zbus::object_server::{InterfaceRef, SignalEmitter};

pub const PATH: &str = "/StatusNotifierItem";

type Pixmaps = Vec<(i32, i32, Vec<u8>)>;

/// A method the tray called on the item, with its arguments.
#[derive(Debug, PartialEq, Eq)]
pub enum Call {
    Activate(i32, i32),
    ContextMenu(i32, i32),
    Scroll(i32, String),
}

/// A property the case can change behind the item's back.
type Shared = Arc<Mutex<String>>;

fn read(shared: &Shared) -> String {
    shared
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

fn write(shared: &Shared, value: &str) {
    *shared.lock().unwrap_or_else(PoisonError::into_inner) = value.to_string();
}

struct Item {
    title: Shared,
    status: Shared,
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
        read(&self.title)
    }

    #[zbus(property)]
    fn status(&self) -> String {
        read(&self.status)
    }

    #[allow(clippy::unused_self)]
    #[zbus(property)]
    fn category(&self) -> String {
        "Communications".to_string()
    }

    #[allow(clippy::unused_self)]
    #[zbus(property)]
    fn id(&self) -> String {
        "fake-item".to_string()
    }

    #[allow(clippy::unused_self)]
    #[zbus(property)]
    const fn item_is_menu(&self) -> bool {
        false
    }

    #[allow(clippy::unused_self)]
    #[zbus(property)]
    fn tool_tip(&self) -> (String, Pixmaps, String, String) {
        (
            String::new(),
            Pixmaps::new(),
            "<b>Tip</b>".to_string(),
            "a &amp; b".to_string(),
        )
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
}

/// The fake item's own connection to the private bus.
pub struct ItemService {
    connection: zbus::Connection,
    title: Shared,
    status: Shared,
    calls: mpsc::UnboundedReceiver<Call>,
}

impl ItemService {
    pub async fn start() -> Outcome<Self> {
        let title = Arc::new(Mutex::new("Fake Title".to_string()));
        let status = Arc::new(Mutex::new("Active".to_string()));
        let (sender, calls) = mpsc::unbounded_channel();
        let item = Item {
            title: Arc::clone(&title),
            status: Arc::clone(&status),
            calls: sender,
        };
        let connection = zbus::connection::Builder::session()
            .context("item: session builder")?
            .serve_at(PATH, item)
            .context("item: serve")?
            .build()
            .await
            .context("item: connect")?;
        Ok(Self {
            connection,
            title,
            status,
            calls,
        })
    }

    /// The id the tray gives an item that registers by object path.
    pub fn tray_id(&self) -> Outcome<String> {
        let name = self.connection.unique_name().context("item: unique name")?;
        Ok(format!("{name}{PATH}"))
    }

    /// Registers with the watcher by object path, as real items do.
    pub async fn register(&self) -> Outcome {
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

    /// Changes `Title` and announces it with `NewTitle`.
    pub async fn retitle(&self, title: &str) -> Outcome {
        write(&self.title, title);
        let interface = self.interface().await?;
        Item::new_title(interface.signal_emitter())
            .await
            .context("item: emit NewTitle")
    }

    /// Changes `Status` and announces it with `NewStatus`.
    pub async fn restatus(&self, status: &str) -> Outcome {
        write(&self.status, status);
        let interface = self.interface().await?;
        Item::new_status(interface.signal_emitter(), status)
            .await
            .context("item: emit NewStatus")
    }

    /// The next method the tray called on the item.
    pub async fn next_call(&mut self) -> Option<Call> {
        self.calls.recv().await
    }

    /// Leaves the bus, as an application that quit.
    pub async fn disconnect(self) -> Outcome {
        self.connection.close().await.context("item: disconnect")
    }
}
