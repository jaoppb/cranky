//! A fake `org.kde.StatusNotifierItem` on the private bus.
//!
//! Plain Rust types only, so the `zbus` bump of #30 has nothing to migrate.

use super::themes::{THEMED_ICON, Theme, Themes};
use crate::support::{Context, Outcome};
use tokio::sync::mpsc;
use zbus::OwnedObjectPath;
use zbus::object_server::{InterfaceRef, SignalEmitter};

pub(crate) const PATH: &str = "/StatusNotifierItem";

/// The `Menu` the item starts with.
const MENU: &str = "/Menu";

type Pixmaps = Vec<(i32, i32, Vec<u8>)>;

/// A method the tray called on the item, with its arguments.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Call {
    Activate(i32, i32),
    SecondaryActivate(i32, i32),
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
    menu: OwnedObjectPath,
    tool_tip: (String, Pixmaps, String, String),
    icon_name: String,
    icon_theme_path: String,
    attention_icon_name: String,
    attention_icon_theme_path: String,
    overlay_icon_name: String,
    /// The methods the item answers with an error, after recording the call.
    rejects: Vec<&'static str>,
    calls: mpsc::UnboundedSender<Call>,
}

impl Item {
    /// Records `call`, then rejects it when `method` is one of `rejects`.
    fn answer(&self, method: &str, call: Call) -> zbus::fdo::Result<()> {
        // The receiver only goes away when the case is over.
        let _ = self.calls.send(call);
        if self.rejects.contains(&method) {
            Err(zbus::fdo::Error::Failed(format!("{method} rejected")))
        } else {
            Ok(())
        }
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
    fn menu(&self) -> OwnedObjectPath {
        self.menu.clone()
    }

    #[zbus(property)]
    fn tool_tip(&self) -> (String, Pixmaps, String, String) {
        self.tool_tip.clone()
    }

    #[zbus(property)]
    fn icon_name(&self) -> String {
        self.icon_name.clone()
    }

    #[zbus(property)]
    fn icon_theme_path(&self) -> String {
        self.icon_theme_path.clone()
    }

    #[zbus(property)]
    fn attention_icon_name(&self) -> String {
        self.attention_icon_name.clone()
    }

    #[zbus(property)]
    fn attention_icon_theme_path(&self) -> String {
        self.attention_icon_theme_path.clone()
    }

    #[zbus(property)]
    fn overlay_icon_name(&self) -> String {
        self.overlay_icon_name.clone()
    }

    fn activate(&self, x: i32, y: i32) -> zbus::fdo::Result<()> {
        self.answer("Activate", Call::Activate(x, y))
    }

    fn secondary_activate(&self, x: i32, y: i32) -> zbus::fdo::Result<()> {
        self.answer("SecondaryActivate", Call::SecondaryActivate(x, y))
    }

    fn context_menu(&self, x: i32, y: i32) -> zbus::fdo::Result<()> {
        self.answer("ContextMenu", Call::ContextMenu(x, y))
    }

    fn scroll(&self, delta: i32, orientation: String) -> zbus::fdo::Result<()> {
        self.answer("Scroll", Call::Scroll(delta, orientation))
    }

    #[zbus(signal)]
    async fn new_title(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_status(emitter: &SignalEmitter<'_>, status: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_menu(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_icon(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_icon_theme_path(emitter: &SignalEmitter<'_>, path: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_attention_icon(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_overlay_icon(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_tool_tip(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// The fake item's own connection to the private bus.
pub(crate) struct ItemService {
    connection: zbus::Connection,
    calls: mpsc::UnboundedReceiver<Call>,
    themes: Themes,
}

impl ItemService {
    /// Serves an activatable item whose icon only resolves in the extra
    /// theme, with no attention or overlay icon, the `/Menu` menu and a tool
    /// tip.
    pub(crate) async fn start() -> Outcome<Self> {
        let (sender, calls) = mpsc::unbounded_channel();
        let themes = Themes::create()?;
        let item = initial_item(&themes, sender)?;
        let connection = zbus::connection::Builder::session()
            .serve_at(PATH, item)
            .build()
            .await
            .context("item: connect")?;
        Ok(Self {
            connection,
            calls,
            themes,
        })
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

    /// Makes the item answer each of `methods` with an error from now on.
    pub(crate) async fn reject(&self, methods: &[&'static str]) -> Outcome {
        self.interface().await?.get_mut().await.rejects = methods.to_vec();
        Ok(())
    }

    /// Changes `IconName` and announces it with `NewIcon`.
    pub(crate) async fn rename_icon(&self, name: &str) -> Outcome {
        let interface = self.interface().await?;
        interface.get_mut().await.icon_name = name.to_string();
        Item::new_icon(interface.signal_emitter())
            .await
            .context("item: emit NewIcon")
    }

    /// Moves `IconThemePath` to `theme` and announces it with
    /// `NewIconThemePath`.
    pub(crate) async fn move_theme(&self, theme: Theme) -> Outcome {
        let interface = self.interface().await?;
        let path = self.themes.path(theme);
        interface.get_mut().await.icon_theme_path.clone_from(&path);
        Item::new_icon_theme_path(interface.signal_emitter(), &path)
            .await
            .context("item: emit NewIconThemePath")
    }

    /// Changes `AttentionIconName` and announces it with `NewAttentionIcon`.
    pub(crate) async fn rename_attention_icon(&self, name: &str) -> Outcome {
        let interface = self.interface().await?;
        interface.get_mut().await.attention_icon_name = name.to_string();
        Item::new_attention_icon(interface.signal_emitter())
            .await
            .context("item: emit NewAttentionIcon")
    }

    /// Changes `OverlayIconName` and announces it with `NewOverlayIcon`.
    pub(crate) async fn rename_overlay_icon(&self, name: &str) -> Outcome {
        let interface = self.interface().await?;
        interface.get_mut().await.overlay_icon_name = name.to_string();
        Item::new_overlay_icon(interface.signal_emitter())
            .await
            .context("item: emit NewOverlayIcon")
    }

    /// Changes the title of `ToolTip` and announces it with `NewToolTip`.
    pub(crate) async fn retip(&self, title: &str) -> Outcome {
        let interface = self.interface().await?;
        interface.get_mut().await.tool_tip.2 = title.to_string();
        Item::new_tool_tip(interface.signal_emitter())
            .await
            .context("item: emit NewToolTip")
    }

    /// Changes `Menu` and announces it with `NewMenu`.
    pub(crate) async fn move_menu(&self, path: &str) -> Outcome {
        let interface = self.interface().await?;
        interface.get_mut().await.menu = object_path(path)?;
        Item::new_menu(interface.signal_emitter())
            .await
            .context("item: emit NewMenu")
    }

    /// The next method the tray called on the item.
    pub(crate) async fn next_call(&mut self) -> Option<Call> {
        self.calls.recv().await
    }

    /// A call already recorded and not taken yet, if any.
    pub(crate) fn pending_call(&mut self) -> Option<Call> {
        self.calls.try_recv().ok()
    }

    /// Leaves the bus, as an application that quit.
    pub(crate) async fn disconnect(self) -> Outcome {
        self.connection.close().await.context("item: disconnect")
    }
}

fn object_path(path: &str) -> Outcome<OwnedObjectPath> {
    OwnedObjectPath::try_from(path).context("item: object path")
}

/// The fake item's state before any test changes it: an activatable item whose icon only
/// resolves in the extra theme, with no attention or overlay icon, the `/Menu` menu and a tool tip,
/// reporting its calls on `calls`.
fn initial_item(themes: &Themes, calls: mpsc::UnboundedSender<Call>) -> Outcome<Item> {
    Ok(Item {
        title: "Fake Title".to_string(),
        status: "Active".to_string(),
        category: "Communications",
        id: "fake-item",
        is_menu: ItemIsMenu::Activatable,
        menu: object_path(MENU)?,
        tool_tip: (
            String::new(),
            Pixmaps::new(),
            "<b>Tip</b>".to_string(),
            "a &amp; b".to_string(),
        ),
        icon_name: THEMED_ICON.to_string(),
        icon_theme_path: themes.path(Theme::Main),
        attention_icon_name: String::new(),
        attention_icon_theme_path: themes.path(Theme::Attention),
        overlay_icon_name: String::new(),
        rejects: Vec::new(),
        calls,
    })
}
