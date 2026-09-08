use super::cleaner::parse_raw_tooltip;
use super::icon_resolver::resolve_icon;
use crate::features::systray::domain::{
    CreateSystrayItemCommand, Destination, IconName, ItemId, ItemIsMenu, ObjectPath,
    SystrayCategory, SystrayIcon, SystrayId, SystrayItem, SystrayItemParams, SystrayStatus,
    SystrayTooltip, Title, WindowId,
};
use std::collections::HashMap;
use zbus::Connection;
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::zvariant::{ObjectPath as ZObjectPath, OwnedValue, Value};

async fn resolve_systray_icon(
    name: Option<String>,
    theme_path: Option<String>,
    pixmap: Option<Vec<(i32, i32, Vec<u8>)>>,
) -> Option<SystrayIcon> {
    let image = resolve_icon(name.clone(), theme_path, pixmap).await;
    SystrayIcon::new(name.map(IconName::new), image)
}

async fn resolve_all_icons(
    all_props: &mut HashMap<String, OwnedValue>,
    icon_name: Option<String>,
    theme_path: Option<String>,
) -> (Option<SystrayIcon>, Option<SystrayIcon>, Option<SystrayIcon>) {
    let icon_pixmap = all_props.remove("IconPixmap").and_then(|v| v.try_into().ok());
    let icon = resolve_systray_icon(icon_name, theme_path.clone(), icon_pixmap).await;

    let att_name = all_props
        .remove("AttentionIconName")
        .and_then(|v| v.try_into().ok());
    let att_theme = all_props
        .remove("AttentionIconThemePath")
        .and_then(|v| v.try_into().ok());
    let att_pixmap = all_props
        .remove("AttentionIconPixmap")
        .and_then(|v| v.try_into().ok());
    let attention_icon = resolve_systray_icon(att_name, att_theme, att_pixmap).await;

    let ovr_name = all_props
        .remove("OverlayIconName")
        .and_then(|v| v.try_into().ok());
    let ovr_pixmap = all_props
        .remove("OverlayIconPixmap")
        .and_then(|v| v.try_into().ok());
    let overlay_icon = resolve_systray_icon(ovr_name, theme_path, ovr_pixmap).await;

    (icon, attention_icon, overlay_icon)
}

fn parse_window_id(all_props: &mut HashMap<String, OwnedValue>) -> Option<u32> {
    all_props
        .remove("WindowId")
        .and_then(|v| v.try_into().ok())
        .or_else(|| {
            all_props
                .remove("WindowId")
                .and_then(|v| v.try_into().ok())
                .and_then(|id: i32| u32::try_from(id).ok())
        })
}

fn parse_menu_path(all_props: &mut HashMap<String, OwnedValue>) -> Option<String> {
    all_props
        .remove("Menu")
        .and_then(|v| v.try_into().ok())
        .or_else(|| {
            all_props.remove("Menu").and_then(|v| {
                if let Value::ObjectPath(p) = &*v {
                    Some(p.as_str().to_string())
                } else {
                    None
                }
            })
        })
}

fn parse_status(status_str: &str) -> SystrayStatus {
    match status_str {
        "Active" => SystrayStatus::Active,
        "Passive" => SystrayStatus::Passive,
        "NeedsAttention" => SystrayStatus::NeedsAttention,
        _ => SystrayStatus::Unknown,
    }
}

async fn fetch_all_props(
    conn: &Connection,
    dest: &str,
    path_str: &str,
) -> Option<HashMap<String, OwnedValue>> {
    let iface = InterfaceName::try_from("org.kde.StatusNotifierItem").ok()?;
    let path = ZObjectPath::try_from(path_str).ok()?;
    let props_builder = PropertiesProxy::builder(conn).destination(dest).ok()?;
    let props_builder = props_builder.path(path).ok()?;
    let props = props_builder.build().await.ok()?;
    Some(props.get_all(iface).await.unwrap_or_default())
}

pub async fn fetch_systray_item(
    conn: &Connection,
    id: String,
    dest: String,
    path_str: String,
) -> SystrayItem {
    let default_item = || {
        SystrayItem::new(CreateSystrayItemCommand::new(
            SystrayItemParams::new(
                SystrayId::new(id.clone()),
                Destination::new(dest.clone()),
                ObjectPath::new(path_str.clone()),
                Title::new(String::new()),
            )
            .with_status(SystrayStatus::Unknown)
            .with_category(SystrayCategory::ApplicationStatus)
            .with_item_is_menu(ItemIsMenu::new(false)),
        ))
    };

    let Some(mut all_props) = fetch_all_props(conn, &dest, &path_str).await else {
        return default_item();
    };

    let title: String = all_props
        .remove("Title")
        .and_then(|v| v.try_into().ok())
        .unwrap_or_default();
    let status_str: String = all_props
        .remove("Status")
        .and_then(|v| v.try_into().ok())
        .unwrap_or_default();
    let icon_name: Option<String> = all_props.remove("IconName").and_then(|v| v.try_into().ok());
    let icon_theme_path: Option<String> = all_props
        .remove("IconThemePath")
        .and_then(|v| v.try_into().ok());
    let category_str: String = all_props
        .remove("Category")
        .and_then(|v| v.try_into().ok())
        .unwrap_or_default();
    let item_id: Option<String> = all_props.remove("Id").and_then(|v| v.try_into().ok());
    let window_id = parse_window_id(&mut all_props);
    let item_is_menu_val: bool = all_props
        .remove("ItemIsMenu")
        .and_then(|v| v.try_into().ok())
        .unwrap_or_default();
    let menu_path_str = parse_menu_path(&mut all_props);

    tracing::debug!(
        "SNI fetch [{id}]: title='{title}', status='{status_str}', icon_name='{icon_name:?}', theme_path='{icon_theme_path:?}'"
    );

    let status = parse_status(&status_str);
    let (icon, attention_icon, overlay_icon) =
        resolve_all_icons(&mut all_props, icon_name, icon_theme_path).await;

    let tooltip: Option<SystrayTooltip> = all_props
        .remove("ToolTip")
        .or_else(|| all_props.remove("Tooltip"))
        .and_then(parse_raw_tooltip);

    let cmd = CreateSystrayItemCommand::new(
        SystrayItemParams::new(
            SystrayId::new(id),
            Destination::new(dest),
            ObjectPath::new(path_str),
            Title::new(title),
        )
        .with_status(status)
        .with_icon(icon)
        .with_menu_path(menu_path_str.map(ObjectPath::new))
        .with_category(SystrayCategory::parse_str(&category_str))
        .with_item_is_menu(ItemIsMenu::new(item_is_menu_val)),
    )
    .with_item_id(item_id.map(ItemId::new))
    .with_window_id(window_id.map(WindowId::new))
    .with_attention_icon(attention_icon)
    .with_overlay_icon(overlay_icon)
    .with_tooltip(tooltip);

    SystrayItem::new(cmd)
}
