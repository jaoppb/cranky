#[zbus::proxy(interface = "org.kde.StatusNotifierItem", assume_defaults = true)]
pub trait StatusNotifierItem {
    #[zbus(property(emits_changed_signal = "false"))]
    fn title(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn status(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_name(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_theme_path(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn attention_icon_name(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn attention_icon_theme_path(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn attention_icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn overlay_icon_name(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn overlay_icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn item_is_menu(&self) -> zbus::Result<bool>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn menu(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn tool_tip(&self) -> zbus::Result<zbus::zvariant::OwnedValue>;

    #[zbus(signal)]
    fn new_title(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_icon(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_status(&self, status: String) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_icon_theme_path(&self, path: String) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_attention_icon(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_overlay_icon(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_tool_tip(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_menu(&self) -> zbus::Result<()>;
}
