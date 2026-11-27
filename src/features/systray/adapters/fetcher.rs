use super::cleaner::parse_raw_tooltip;
use super::icon_resolver::resolve_icon;
use crate::features::systray::domain::{
    CreateSystrayItemCommand, IconName, ItemId, ItemIsMenu, ObjectPath, SystrayAddress,
    SystrayCategory, SystrayIcon, SystrayId, SystrayItem, SystrayItemParams, SystrayStatus,
    SystrayTooltip, Title, WindowId,
};
use std::collections::HashMap;
use zbus::Connection;
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::{ObjectPath as ZObjectPath, OwnedVariant, Variant};

/// The properties of an item, as `GetAll` returned them.
type AllProps = HashMap<String, OwnedVariant>;

/// Takes `key` out of the properties as a `T`; a missing or mistyped property is `None`.
fn take<T: TryFrom<OwnedVariant>>(all_props: &mut AllProps, key: &str) -> Option<T> {
    all_props
        .remove(key)
        .and_then(|variant| variant.try_into().ok())
}

/// Resolves an icon's image from its name, theme path and pixmaps, and pairs it with the name.
pub(super) async fn resolve_systray_icon(
    name: Option<String>,
    theme_path: Option<String>,
    pixmap: Option<Vec<(i32, i32, Vec<u8>)>>,
) -> Option<SystrayIcon> {
    let image = resolve_icon(name.clone(), theme_path, pixmap).await;
    SystrayIcon::new(name.map(IconName::new), image)
}

async fn resolve_all_icons(
    all_props: &mut AllProps,
    icon_name: Option<String>,
    theme_path: Option<String>,
) -> (
    Option<SystrayIcon>,
    Option<SystrayIcon>,
    Option<SystrayIcon>,
) {
    let icon_pixmap = take(all_props, "IconPixmap");
    let icon = resolve_systray_icon(icon_name, theme_path.clone(), icon_pixmap).await;

    let att_name = take(all_props, "AttentionIconName");
    let att_theme = take(all_props, "AttentionIconThemePath");
    let att_pixmap = take(all_props, "AttentionIconPixmap");
    let attention_icon = resolve_systray_icon(att_name, att_theme, att_pixmap).await;

    let ovr_name = take(all_props, "OverlayIconName");
    let ovr_pixmap = take(all_props, "OverlayIconPixmap");
    let overlay_icon = resolve_systray_icon(ovr_name, theme_path, ovr_pixmap).await;

    (icon, attention_icon, overlay_icon)
}

fn parse_window_id(all_props: &mut AllProps) -> Option<u32> {
    take(all_props, "WindowId")
        .or_else(|| take(all_props, "WindowId").and_then(|id: i32| u32::try_from(id).ok()))
}

fn parse_menu_path(all_props: &mut AllProps) -> Option<String> {
    take(all_props, "Menu").or_else(|| {
        all_props.remove("Menu").and_then(|variant| {
            if let Variant::ObjectPath(object_path) = &*variant {
                Some(object_path.as_str().to_string())
            } else {
                None
            }
        })
    })
}

/// Maps the SNI `Status` names; anything else, including a different case, is `Unknown`.
pub(super) fn parse_status(status_str: &str) -> SystrayStatus {
    match status_str {
        "Active" => SystrayStatus::Active,
        "Passive" => SystrayStatus::Passive,
        "NeedsAttention" => SystrayStatus::NeedsAttention,
        _ => SystrayStatus::Unknown,
    }
}

async fn fetch_all_props(conn: &Connection, dest: &str, path_str: &str) -> Option<AllProps> {
    let iface = InterfaceName::try_from("org.kde.StatusNotifierItem").ok()?;
    let path = ZObjectPath::try_from(path_str).ok()?;
    let props = PropertiesProxy::builder(conn)
        .destination(dest)
        .path(path)
        .build()
        .await
        .ok()?;
    Some(props.get_all(iface).await.unwrap_or_default())
}

/// The item a tray shows for an address whose properties could not be read: no title, unknown
/// status, an activatable application.
fn default_item(address: SystrayAddress) -> SystrayItem {
    SystrayItem::new(CreateSystrayItemCommand::new(
        SystrayItemParams::new(address.id(), address, Title::new(String::new()))
            .with_status(SystrayStatus::Unknown)
            .with_category(SystrayCategory::ApplicationStatus)
            .with_item_is_menu(ItemIsMenu::Activatable),
    ))
}

/// The plain properties of an item, taken out of its `GetAll` reply; the icons and the tooltip
/// are resolved from what is left.
struct ItemProps {
    title: String,
    status_str: String,
    icon_name: Option<String>,
    icon_theme_path: Option<String>,
    category_str: String,
    item_id: Option<String>,
    window_id: Option<u32>,
    item_is_menu: ItemIsMenu,
    menu_path_str: Option<String>,
}

impl ItemProps {
    /// Takes the plain properties out, each defaulting when missing or mistyped.
    fn take(all_props: &mut AllProps) -> Self {
        Self {
            title: take(all_props, "Title").unwrap_or_default(),
            status_str: take(all_props, "Status").unwrap_or_default(),
            icon_name: take(all_props, "IconName"),
            icon_theme_path: take(all_props, "IconThemePath"),
            category_str: take(all_props, "Category").unwrap_or_default(),
            item_id: take(all_props, "Id"),
            window_id: parse_window_id(all_props),
            item_is_menu: ItemIsMenu::from(
                take::<bool>(all_props, "ItemIsMenu").unwrap_or_default(),
            ),
            menu_path_str: parse_menu_path(all_props),
        }
    }

    fn log_fetched(&self, id: &SystrayId) {
        let Self {
            title,
            status_str,
            icon_name,
            icon_theme_path,
            ..
        } = self;
        tracing::debug!(
            "SNI fetch [{}]: title='{title}', status='{status_str}', icon_name='{icon_name:?}', theme_path='{icon_theme_path:?}'",
            id.as_str()
        );
    }

    /// The command for the item at `address`, with its icons and tooltip resolved from the
    /// properties left in `all_props`.
    async fn into_command(
        self,
        address: SystrayAddress,
        all_props: &mut AllProps,
    ) -> CreateSystrayItemCommand {
        let status = parse_status(&self.status_str);
        let (icon, attention_icon, overlay_icon) =
            resolve_all_icons(all_props, self.icon_name, self.icon_theme_path).await;
        let tooltip: Option<SystrayTooltip> = all_props
            .remove("ToolTip")
            .or_else(|| all_props.remove("Tooltip"))
            .and_then(parse_raw_tooltip);
        CreateSystrayItemCommand::new(
            SystrayItemParams::new(address.id(), address, Title::new(self.title))
                .with_status(status)
                .with_icon(icon)
                .with_menu_path(self.menu_path_str.map(ObjectPath::new))
                .with_category(SystrayCategory::parse_str(&self.category_str))
                .with_item_is_menu(self.item_is_menu),
        )
        .with_item_id(self.item_id.map(ItemId::new))
        .with_window_id(self.window_id.map(WindowId::new))
        .with_attention_icon(attention_icon)
        .with_overlay_icon(overlay_icon)
        .with_tooltip(tooltip)
    }
}

pub(super) async fn fetch_systray_item(conn: &Connection, address: SystrayAddress) -> SystrayItem {
    let Some(mut all_props) = fetch_all_props(
        conn,
        address.destination().as_str(),
        address.path().as_str(),
    )
    .await
    else {
        return default_item(address);
    };
    let props = ItemProps::take(&mut all_props);
    props.log_fetched(&address.id());
    SystrayItem::new(props.into_command(address, &mut all_props).await)
}

#[cfg(test)]
mod tests {
    use super::{
        ItemProps, default_item, parse_menu_path, parse_status, parse_window_id, resolve_all_icons,
    };
    use crate::features::systray::adapters::tests::fixtures::{TempIconDir, first_pixel};
    use crate::features::systray::domain::{
        CreateSystrayItemCommand, Destination, IconImage, IconName, ItemId, ItemIsMenu, ObjectPath,
        SystrayAddress, SystrayCategory, SystrayIcon, SystrayId, SystrayItem, SystrayItemParams,
        SystrayStatus, SystrayTooltip, SystrayTooltipDescription, SystrayTooltipTitle, Title,
        WindowId,
    };
    use std::collections::HashMap;
    use zbus::{ObjectPath as ZObjectPath, OwnedVariant, Variant};

    /// The properties an item reported, from `(name, value)` pairs.
    fn props<'a>(
        pairs: impl IntoIterator<Item = (&'a str, Variant<'a>)>,
    ) -> HashMap<String, OwnedVariant> {
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.try_into().unwrap()))
            .collect()
    }

    #[test]
    fn test_parse_status_maps_the_sni_names() {
        let parsed = ["Active", "Passive", "NeedsAttention", "active", ""].map(parse_status);
        assert_eq!(
            parsed,
            [
                SystrayStatus::Active,
                SystrayStatus::Passive,
                SystrayStatus::NeedsAttention,
                SystrayStatus::Unknown,
                SystrayStatus::Unknown,
            ]
        );
    }

    #[test]
    fn test_parse_window_id_reads_a_u32_and_takes_the_property() {
        let mut all_props = props([("WindowId", Variant::from(7_u32))]);
        assert_eq!(parse_window_id(&mut all_props), Some(7));
        assert!(all_props.is_empty());
        assert_eq!(parse_window_id(&mut all_props), None);
    }

    #[test]
    fn test_parse_window_id_drops_an_i32() {
        // The first attempt already took the property, so the i32 fallback never sees it.
        let mut all_props = props([("WindowId", Variant::from(7_i32))]);
        assert_eq!(parse_window_id(&mut all_props), None);
        assert!(all_props.is_empty());
    }

    #[test]
    fn test_parse_menu_path_reads_a_string_and_takes_the_property() {
        let mut all_props = props([("Menu", Variant::from("/menu"))]);
        assert_eq!(parse_menu_path(&mut all_props), Some("/menu".to_string()));
        assert!(all_props.is_empty());
    }

    #[test]
    fn test_parse_menu_path_of_an_object_path() {
        let path = ZObjectPath::try_from("/menu").unwrap();
        let mut all_props = props([("Menu", Variant::from(path))]);
        let parsed = parse_menu_path(&mut all_props);
        assert!(all_props.is_empty());
        assert_eq!(parsed, OBJECT_PATH_MENU);
    }

    /// What an object-path `Menu` parses to: the string conversion is tried first.
    const OBJECT_PATH_MENU: Option<String> = None;

    fn image(icon: Option<&SystrayIcon>) -> Option<&IconImage> {
        icon.and_then(SystrayIcon::image)
    }

    #[test]
    fn test_resolve_all_icons_reads_each_pixmap_for_its_own_icon() {
        crate::test_utils::block_on(async {
            let mut all_props = props([
                (
                    "IconPixmap",
                    Variant::from(vec![(1, 1, vec![255_u8, 1, 1, 1])]),
                ),
                (
                    "AttentionIconPixmap",
                    Variant::from(vec![(1, 1, vec![255_u8, 2, 2, 2])]),
                ),
                (
                    "OverlayIconPixmap",
                    Variant::from(vec![(1, 1, vec![255_u8, 3, 3, 3])]),
                ),
            ]);
            let (icon, attention, overlay) = resolve_all_icons(&mut all_props, None, None).await;
            let data =
                [&icon, &attention, &overlay].map(|icon| image(icon.as_ref()).map(IconImage::data));
            assert_eq!(
                data,
                [
                    Some(&[1, 1, 1, 255][..]),
                    Some(&[2, 2, 2, 255][..]),
                    Some(&[3, 3, 3, 255][..])
                ]
            );
            assert!(all_props.is_empty());
        });
    }

    #[test]
    fn test_resolve_all_icons_looks_each_name_up_in_its_theme_path() {
        crate::test_utils::block_on(async {
            let main = TempIconDir::new("fetch-main-theme");
            let attention_dir = TempIconDir::new("fetch-attention-theme");
            main.svg("cranky-fetch-main", "#ff0000");
            main.svg("cranky-fetch-overlay", "#0000ff");
            attention_dir.svg("cranky-fetch-attention", "#00ff00");
            let attention_theme = attention_dir.theme_path();
            let mut all_props = props([
                ("AttentionIconName", Variant::from("cranky-fetch-attention")),
                (
                    "AttentionIconThemePath",
                    Variant::from(attention_theme.as_str()),
                ),
                ("OverlayIconName", Variant::from("cranky-fetch-overlay")),
            ]);
            let (icon, attention, overlay) = resolve_all_icons(
                &mut all_props,
                Some("cranky-fetch-main".to_string()),
                Some(main.theme_path()),
            )
            .await;
            let names = [&icon, &attention, &overlay].map(|icon| {
                icon.as_ref()
                    .and_then(SystrayIcon::name)
                    .map(IconName::as_str)
            });
            assert_eq!(
                names,
                [
                    Some("cranky-fetch-main"),
                    Some("cranky-fetch-attention"),
                    Some("cranky-fetch-overlay")
                ]
            );
            let pixels =
                [&icon, &attention, &overlay].map(|icon| image(icon.as_ref()).map(first_pixel));
            assert_eq!(
                pixels,
                [
                    Some(&[255, 0, 0, 255][..]),
                    Some(&[0, 255, 0, 255][..]),
                    Some(&[0, 0, 255, 255][..])
                ]
            );
            assert!(all_props.is_empty());
        });
    }

    fn address() -> SystrayAddress {
        SystrayAddress::new(
            Destination::new(":1.9"),
            ObjectPath::new("/StatusNotifierItem"),
        )
    }

    /// The item the fetcher builds from `all_props`, as `fetch_systray_item` does once
    /// `GetAll` answered.
    fn fetched(mut all_props: HashMap<String, OwnedVariant>) -> SystrayItem {
        crate::test_utils::block_on(async {
            let item_props = ItemProps::take(&mut all_props);
            SystrayItem::new(item_props.into_command(address(), &mut all_props).await)
        })
    }

    /// The command every field of `fetched` is checked against: `title`, at `address()`.
    fn params(title: &str) -> SystrayItemParams {
        SystrayItemParams::new(
            SystrayId::new(":1.9/StatusNotifierItem"),
            address(),
            Title::new(title),
        )
    }

    /// Every property an item can report, each set to something other than its default.
    fn full_props() -> HashMap<String, OwnedVariant> {
        let tool_tip = (
            String::new(),
            Vec::<(i32, i32, Vec<u8>)>::new(),
            "<b>T</b>",
            "a &amp; b",
        );
        props([
            ("Title", Variant::from("Chat")),
            ("Status", Variant::from("NeedsAttention")),
            ("IconName", Variant::from("cranky-fetch-missing-icon")),
            ("Category", Variant::from("Communications")),
            ("Id", Variant::from("chat-app")),
            ("WindowId", Variant::from(12_u32)),
            ("ItemIsMenu", Variant::from(true)),
            ("Menu", Variant::from("/MenuBar")),
            ("ToolTip", Variant::from(tool_tip)),
            (
                "AttentionIconName",
                Variant::from("cranky-fetch-missing-attention"),
            ),
            (
                "OverlayIconName",
                Variant::from("cranky-fetch-missing-overlay"),
            ),
        ])
    }

    /// The item `full_props` describes; no icon file exists, so each icon is its name alone.
    fn full_item() -> SystrayItem {
        let named = |name: &str| SystrayIcon::new(Some(IconName::new(name)), None);
        let tip_title = SystrayTooltipTitle::new("T");
        let tooltip = SystrayTooltip::new(None, tip_title, SystrayTooltipDescription::new("a & b"));
        SystrayItem::new(
            CreateSystrayItemCommand::new(
                params("Chat")
                    .with_status(SystrayStatus::NeedsAttention)
                    .with_icon(named("cranky-fetch-missing-icon"))
                    .with_menu_path(Some(ObjectPath::new("/MenuBar")))
                    .with_category(SystrayCategory::Communications)
                    .with_item_is_menu(ItemIsMenu::MenuOnly),
            )
            .with_item_id(Some(ItemId::new("chat-app")))
            .with_window_id(Some(WindowId::new(12)))
            .with_attention_icon(named("cranky-fetch-missing-attention"))
            .with_overlay_icon(named("cranky-fetch-missing-overlay"))
            .with_tooltip(Some(tooltip)),
        )
    }

    #[test]
    fn test_fetched_item_carries_every_property() {
        assert_eq!(fetched(full_props()), full_item());
    }

    #[test]
    fn test_fetched_item_defaults_missing_properties() {
        let expected = CreateSystrayItemCommand::new(
            params("")
                .with_status(SystrayStatus::Unknown)
                .with_category(SystrayCategory::Other(String::new()))
                .with_item_is_menu(ItemIsMenu::Activatable),
        );
        assert_eq!(fetched(HashMap::new()), SystrayItem::new(expected));
    }

    #[test]
    fn test_fetched_item_reads_the_lowercase_tooltip_key() {
        let tool_tip = (String::new(), Vec::<(i32, i32, Vec<u8>)>::new(), "Tip", "");
        let item = fetched(props([("Tooltip", Variant::from(tool_tip))]));
        let title = item.tooltip().map(|tooltip| tooltip.title().as_str());
        assert_eq!(title, Some("Tip"));
    }

    #[test]
    fn test_default_item_is_an_unknown_activatable_application() {
        let expected = CreateSystrayItemCommand::new(
            params("")
                .with_status(SystrayStatus::Unknown)
                .with_category(SystrayCategory::ApplicationStatus)
                .with_item_is_menu(ItemIsMenu::Activatable),
        );
        assert_eq!(default_item(address()), SystrayItem::new(expected));
    }
}
