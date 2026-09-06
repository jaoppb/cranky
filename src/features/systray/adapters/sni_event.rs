use super::cleaner::parse_raw_tooltip;
use super::icon_resolver::resolve_icon;
use super::sni_proxy::StatusNotifierItemProxy;
use crate::features::systray::domain::{
    IconName, ItemIsMenu, ObjectPath, SystrayIcon, SystrayItem, SystrayStatus,
};

#[derive(Debug)]
pub enum SniEvent {
    Title,
    Status(String),
    Icon,
    ThemePath,
    AttentionIcon,
    OverlayIcon,
    ToolTip,
    Menu,
}

impl SniEvent {
    pub async fn apply(self, item: SystrayItem, proxy: &StatusNotifierItemProxy<'_>) -> SystrayItem {
        match self {
            Self::Title => {
                let title = proxy.title().await.unwrap_or_default();
                tracing::trace!("SniEvent::Title: updated title to '{title}'");
                item.with_title(title)
            }
            Self::Status(status_str) => {
                let status = match status_str.as_str() {
                    "Active" => SystrayStatus::Active,
                    "Passive" => SystrayStatus::Passive,
                    "NeedsAttention" => SystrayStatus::NeedsAttention,
                    _ => SystrayStatus::Unknown,
                };
                tracing::trace!("SniEvent::Status: updated status to {status:?}");
                item.with_status(status)
            }
            Self::Icon | Self::ThemePath => {
                let icon_name = proxy.icon_name().await.ok();
                let icon_theme_path = proxy.icon_theme_path().await.ok();
                let icon_pixmap = proxy.icon_pixmap().await.ok();
                let has_pixmap = icon_pixmap.as_ref().is_some_and(|p| !p.is_empty());
                tracing::trace!(
                    "SniEvent::Icon/ThemePath: updated icon_name={icon_name:?}, theme_path={icon_theme_path:?}, has_pixmap={has_pixmap}"
                );
                let icon_image =
                    resolve_icon(icon_name.clone(), icon_theme_path, icon_pixmap).await;
                let icon = SystrayIcon::new(
                    icon_name.map(IconName::new),
                    icon_image,
                );
                item.with_icon(icon)
            }
            Self::AttentionIcon => {
                let icon_name = proxy.attention_icon_name().await.ok();
                let icon_theme_path = proxy.attention_icon_theme_path().await.ok();
                let icon_pixmap = proxy.attention_icon_pixmap().await.ok();
                tracing::trace!("SniEvent::AttentionIcon: updated attention icon");
                let icon_image =
                    resolve_icon(icon_name.clone(), icon_theme_path, icon_pixmap).await;
                let icon = SystrayIcon::new(
                    icon_name.map(IconName::new),
                    icon_image,
                );
                item.with_attention_icon(icon)
            }
            Self::OverlayIcon => {
                let icon_name = proxy.overlay_icon_name().await.ok();
                let icon_theme_path = proxy.icon_theme_path().await.ok();
                let icon_pixmap = proxy.overlay_icon_pixmap().await.ok();
                tracing::trace!("SniEvent::OverlayIcon: updated overlay icon");
                let icon_image =
                    resolve_icon(icon_name.clone(), icon_theme_path, icon_pixmap).await;
                let icon = SystrayIcon::new(
                    icon_name.map(IconName::new),
                    icon_image,
                );
                item.with_overlay_icon(icon)
            }
            Self::ToolTip => {
                let tooltip = proxy
                    .tool_tip()
                    .await
                    .ok()
                    .and_then(parse_raw_tooltip);
                tracing::trace!("SniEvent::ToolTip: updated tooltip");
                item.with_tooltip(tooltip)
            }
            Self::Menu => {
                let mut item = item;
                if let Ok(menu_path) = proxy.menu().await {
                    item = item.with_menu_path(Some(
                        ObjectPath::new(menu_path.as_str()),
                    ));
                }
                if let Ok(item_is_menu_val) = proxy.item_is_menu().await {
                    item = item.with_item_is_menu(
                        ItemIsMenu::new(item_is_menu_val),
                    );
                }
                tracing::trace!("SniEvent::Menu: updated menu");
                item
            }
        }
    }
}
