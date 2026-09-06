use super::cleaner::parse_raw_tooltip;
use super::icon_resolver::resolve_icon;
use crate::features::systray::domain::{
    CreateSystrayItemCommand, Destination, IconName, ItemId, ItemIsMenu, ObjectPath, SystrayCategory,
    SystrayIcon, SystrayId, SystrayItem, SystrayStatus, SystrayTooltip, Title, WindowId,
};
use zbus::Connection;
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::zvariant::ObjectPath as ZObjectPath;

#[allow(clippy::too_many_lines)]
pub async fn fetch_systray_item(
    conn: &Connection,
    id: String,
    dest: String,
    path_str: String,
) -> SystrayItem {
    let default_item = || {
        SystrayItem::new(CreateSystrayItemCommand::new(
            SystrayId::new(id.clone()),
            Destination::new(dest.clone()),
            ObjectPath::new(path_str.clone()),
            Title::new(String::new()),
            SystrayStatus::Unknown,
            None,
            None,
            SystrayCategory::ApplicationStatus,
            ItemIsMenu::new(false),
        ))
    };

    let Ok(iface) = InterfaceName::try_from("org.kde.StatusNotifierItem") else {
        return default_item();
    };
    let Ok(path) = ZObjectPath::try_from(path_str.as_str()) else {
        return default_item();
    };

    let Ok(props_builder) = PropertiesProxy::builder(conn).destination(dest.clone()) else {
        return default_item();
    };
    let Ok(props_builder) = props_builder.path(path) else {
        return default_item();
    };
    let Ok(props) = props_builder.build().await else {
        return default_item();
    };

    let mut all_props = props.get_all(iface).await.unwrap_or_default();

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
    let window_id: Option<u32> = all_props
        .remove("WindowId")
        .and_then(|v| v.try_into().ok())
        .or_else(|| {
            all_props
                .remove("WindowId")
                .and_then(|v| v.try_into().ok())
                .and_then(|id: i32| u32::try_from(id).ok())
        });
    let item_is_menu_val: bool = all_props
        .remove("ItemIsMenu")
        .and_then(|v| v.try_into().ok())
        .unwrap_or_default();
    let menu_path_str: Option<String> = all_props
        .remove("Menu")
        .and_then(|v| v.try_into().ok())
        .or_else(|| {
            all_props.remove("Menu").and_then(|v| {
                if let zbus::zvariant::Value::ObjectPath(p) = &*v {
                    Some(p.as_str().to_string())
                } else {
                    None
                }
            })
        });

    tracing::debug!(
        "SNI fetch [{id}]: title='{title}', status='{status_str}', icon_name='{icon_name:?}', theme_path='{icon_theme_path:?}'"
    );

    let status = match status_str.as_str() {
        "Active" => SystrayStatus::Active,
        "Passive" => SystrayStatus::Passive,
        "NeedsAttention" => SystrayStatus::NeedsAttention,
        _ => SystrayStatus::Unknown,
    };

    let icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>> =
        all_props.remove("IconPixmap").and_then(|v| v.try_into().ok());
    let icon_image =
        resolve_icon(icon_name.clone(), icon_theme_path.clone(), icon_pixmap).await;
    let icon = SystrayIcon::new(
        icon_name.map(IconName::new),
        icon_image,
    );

    let attention_icon_name: Option<String> = all_props
        .remove("AttentionIconName")
        .and_then(|v| v.try_into().ok());
    let attention_icon_theme_path: Option<String> = all_props
        .remove("AttentionIconThemePath")
        .and_then(|v| v.try_into().ok());
    let attention_icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>> = all_props
        .remove("AttentionIconPixmap")
        .and_then(|v| v.try_into().ok());
    let attention_icon_image = resolve_icon(
        attention_icon_name.clone(),
        attention_icon_theme_path,
        attention_icon_pixmap,
    )
    .await;
    let attention_icon = SystrayIcon::new(
        attention_icon_name.map(IconName::new),
        attention_icon_image,
    );

    let overlay_icon_name: Option<String> = all_props
        .remove("OverlayIconName")
        .and_then(|v| v.try_into().ok());
    let overlay_icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>> = all_props
        .remove("OverlayIconPixmap")
        .and_then(|v| v.try_into().ok());
    let overlay_icon_image = resolve_icon(
        overlay_icon_name.clone(),
        icon_theme_path,
        overlay_icon_pixmap,
    )
    .await;
    let overlay_icon = SystrayIcon::new(
        overlay_icon_name.map(IconName::new),
        overlay_icon_image,
    );

    let tooltip: Option<SystrayTooltip> = all_props
        .remove("ToolTip")
        .or_else(|| all_props.remove("Tooltip"))
        .and_then(parse_raw_tooltip);

    let cmd = CreateSystrayItemCommand::new(
        SystrayId::new(id),
        Destination::new(dest),
        ObjectPath::new(path_str),
        Title::new(title),
        status,
        icon,
        menu_path_str.map(ObjectPath::new),
        SystrayCategory::parse_str(&category_str),
        ItemIsMenu::new(item_is_menu_val),
    )
    .with_item_id(item_id.map(ItemId::new))
    .with_window_id(window_id.map(WindowId::new))
    .with_attention_icon(attention_icon)
    .with_overlay_icon(overlay_icon)
    .with_tooltip(tooltip);

    SystrayItem::new(cmd)
}
