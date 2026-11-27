use super::cleaner::parse_raw_tooltip;
use super::fetcher::{parse_status, resolve_systray_icon};
use super::sni_proxy::StatusNotifierItemProxy;
use crate::features::systray::domain::{ItemIsMenu, ObjectPath, SystrayItem, SystrayStatus};

#[derive(Debug)]
pub(super) enum SniEvent {
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
    pub(crate) async fn apply(
        self,
        item: SystrayItem,
        proxy: &StatusNotifierItemProxy<'_>,
    ) -> SystrayItem {
        match self {
            Self::Title => apply_title(item, proxy).await,
            Self::Status(status_str) => apply_status(item, &status_str),
            Self::Icon | Self::ThemePath => apply_icon(item, proxy).await,
            Self::AttentionIcon => apply_attention_icon(item, proxy).await,
            Self::OverlayIcon => apply_overlay_icon(item, proxy).await,
            Self::ToolTip => apply_tool_tip(item, proxy).await,
            Self::Menu => apply_menu(item, proxy).await,
        }
    }
}

/// Refetches `Title`; a failed read clears it.
async fn apply_title(item: SystrayItem, proxy: &StatusNotifierItemProxy<'_>) -> SystrayItem {
    let title = proxy.title().await.unwrap_or_default();
    log_title(&title);
    item.with_title(title)
}

/// Takes the status the `NewStatus` signal carried.
fn apply_status(item: SystrayItem, status_str: &str) -> SystrayItem {
    let status = parse_status(status_str);
    log_status(&status);
    item.with_status(status)
}

/// Refetches `IconName`, `IconThemePath` and `IconPixmap`, for a new icon or a new theme path.
async fn apply_icon(item: SystrayItem, proxy: &StatusNotifierItemProxy<'_>) -> SystrayItem {
    let icon_name = proxy.icon_name().await.ok();
    let icon_theme_path = proxy.icon_theme_path().await.ok();
    let icon_pixmap = proxy.icon_pixmap().await.ok();
    log_icon(
        icon_name.as_deref(),
        icon_theme_path.as_deref(),
        icon_pixmap.as_deref(),
    );
    item.with_icon(resolve_systray_icon(icon_name, icon_theme_path, icon_pixmap).await)
}

/// Refetches the attention icon's name, theme path and pixmap.
async fn apply_attention_icon(
    item: SystrayItem,
    proxy: &StatusNotifierItemProxy<'_>,
) -> SystrayItem {
    let icon_name = proxy.attention_icon_name().await.ok();
    let icon_theme_path = proxy.attention_icon_theme_path().await.ok();
    let icon_pixmap = proxy.attention_icon_pixmap().await.ok();
    log_attention_icon();
    item.with_attention_icon(resolve_systray_icon(icon_name, icon_theme_path, icon_pixmap).await)
}

/// Refetches the overlay icon's name and pixmap, looked up in the item's `IconThemePath`.
async fn apply_overlay_icon(item: SystrayItem, proxy: &StatusNotifierItemProxy<'_>) -> SystrayItem {
    let icon_name = proxy.overlay_icon_name().await.ok();
    let icon_theme_path = proxy.icon_theme_path().await.ok();
    let icon_pixmap = proxy.overlay_icon_pixmap().await.ok();
    log_overlay_icon();
    item.with_overlay_icon(resolve_systray_icon(icon_name, icon_theme_path, icon_pixmap).await)
}

/// Refetches `ToolTip`; a failed or malformed read clears it.
async fn apply_tool_tip(item: SystrayItem, proxy: &StatusNotifierItemProxy<'_>) -> SystrayItem {
    let tooltip = proxy.tool_tip().await.ok().and_then(parse_raw_tooltip);
    log_tool_tip();
    item.with_tooltip(tooltip)
}

/// Refetches `Menu` and `ItemIsMenu`, keeping the current value of whichever read fails.
async fn apply_menu(item: SystrayItem, proxy: &StatusNotifierItemProxy<'_>) -> SystrayItem {
    let mut item = item;
    if let Ok(menu_path) = proxy.menu().await {
        item = item.with_menu_path(Some(ObjectPath::new(menu_path.as_str())));
    }
    if let Ok(item_is_menu_val) = proxy.item_is_menu().await {
        item = item.with_item_is_menu(ItemIsMenu::from(item_is_menu_val));
    }
    log_menu();
    item
}

fn log_title(title: &str) {
    tracing::trace!("SniEvent::Title: updated title to '{title}'");
}

fn log_status(status: &SystrayStatus) {
    tracing::trace!("SniEvent::Status: updated status to {status:?}");
}

fn log_icon(
    icon_name: Option<&str>,
    icon_theme_path: Option<&str>,
    icon_pixmap: Option<&[(i32, i32, Vec<u8>)]>,
) {
    let has_pixmap = icon_pixmap.is_some_and(|pixmaps| !pixmaps.is_empty());
    tracing::trace!(
        "SniEvent::Icon/ThemePath: updated icon_name={icon_name:?}, theme_path={icon_theme_path:?}, has_pixmap={has_pixmap}"
    );
}

fn log_attention_icon() {
    tracing::trace!("SniEvent::AttentionIcon: updated attention icon");
}

fn log_overlay_icon() {
    tracing::trace!("SniEvent::OverlayIcon: updated overlay icon");
}

fn log_tool_tip() {
    tracing::trace!("SniEvent::ToolTip: updated tooltip");
}

fn log_menu() {
    tracing::trace!("SniEvent::Menu: updated menu");
}
