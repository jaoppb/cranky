//! Icon theme directories for the fake item, under the sandbox's runtime
//! directory, each holding the icons only it should resolve.

use crate::support::{Context, Outcome};
use std::path::PathBuf;

/// In the main theme: the icon `NewIcon` switches to.
pub(crate) const MAIN_ICON: &str = "cranky-it-icon";
/// In the main theme only: the overlay icon, resolved in `IconThemePath`.
pub(crate) const OVERLAY_ICON: &str = "cranky-it-overlay";
/// In the attention theme only, resolved in `AttentionIconThemePath`.
pub(crate) const ATTENTION_ICON: &str = "cranky-it-attention";
/// In the extra theme only: the icon the item starts with, which only
/// resolves once `IconThemePath` moves there.
pub(crate) const THEMED_ICON: &str = "cranky-it-themed";

/// One of the theme directories.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Theme {
    /// `IconThemePath` at first.
    Main,
    /// `AttentionIconThemePath`.
    Attention,
    /// Where `NewIconThemePath` moves `IconThemePath`.
    Extra,
}

impl Theme {
    const fn dir(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Attention => "attention",
            Self::Extra => "extra",
        }
    }

    const fn icons(self) -> &'static [&'static str] {
        match self {
            Self::Main => &[MAIN_ICON, OVERLAY_ICON],
            Self::Attention => &[ATTENTION_ICON],
            Self::Extra => &[THEMED_ICON],
        }
    }
}

const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="#ff0000"/></svg>"##;

/// The theme directories, removed on drop.
pub(crate) struct Themes(PathBuf);

impl Themes {
    /// Writes every theme directory and its icons.
    pub(crate) fn create() -> Outcome<Self> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR is unset")?;
        let themes = Self(PathBuf::from(runtime).join("item-themes"));
        for theme in [Theme::Main, Theme::Attention, Theme::Extra] {
            let dir = themes.0.join(theme.dir());
            std::fs::create_dir_all(&dir).context("create a theme directory")?;
            for icon in theme.icons() {
                std::fs::write(dir.join(format!("{icon}.svg")), SVG).context("write an icon")?;
            }
        }
        Ok(themes)
    }

    /// The directory of `theme`, as an item reports it.
    pub(crate) fn path(&self, theme: Theme) -> String {
        self.0.join(theme.dir()).to_string_lossy().into_owned()
    }
}

impl Drop for Themes {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
