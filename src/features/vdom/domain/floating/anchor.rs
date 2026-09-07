use serde::{Deserialize, Serialize};

/// Wayland Layer-Shell anchor configuration for floating panels.
///
/// Under the Wayland layer-shell protocol (`zwlr_layer_surface_v1::Anchor`),
/// anchor edges are bitfield flags (`TOP`, `BOTTOM`, `LEFT`, `RIGHT`) rather
/// than mutually exclusive directions. Multiple edges can be pinned simultaneously
/// to control stretching and placement:
/// - `top + left + right`: stretches across the top of the monitor.
/// - `top + bottom + left`: stretches vertically along the left edge.
/// - `all`: stretches across the entire monitor (fullscreen overlay).
/// - `none`: centers the panel on screen.
///
/// Four independent booleans represent these edge flags directly, which requires
/// suppressing `clippy::struct_excessive_bools`.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "PanelAnchorHelper")]
pub struct PanelAnchor {
    top: bool,
    bottom: bool,
    left: bool,
    right: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PanelAnchorHelper {
    Flags {
        #[serde(default)]
        top: bool,
        #[serde(default)]
        bottom: bool,
        #[serde(default)]
        left: bool,
        #[serde(default)]
        right: bool,
    },
    List(Vec<String>),
    Single(String),
}

impl From<PanelAnchorHelper> for PanelAnchor {
    fn from(helper: PanelAnchorHelper) -> Self {
        match helper {
            PanelAnchorHelper::Flags {
                top,
                bottom,
                left,
                right,
            } => Self::new(top, bottom, left, right),
            PanelAnchorHelper::List(list) => {
                let mut top = false;
                let mut bottom = false;
                let mut left = false;
                let mut right = false;
                for item in list {
                    match item.to_lowercase().as_str() {
                        "top" => top = true,
                        "bottom" => bottom = true,
                        "left" => left = true,
                        "right" => right = true,
                        _ => {}
                    }
                }
                Self::new(top, bottom, left, right)
            }
            PanelAnchorHelper::Single(s) => match s.to_lowercase().as_str() {
                "top" => Self::new(true, false, false, false),
                "bottom" => Self::new(false, true, false, false),
                "left" => Self::new(false, false, true, false),
                "right" => Self::new(false, false, false, true),
                "top_left" | "top-left" => Self::new(true, false, true, false),
                "top_right" | "top-right" => Self::new(true, false, false, true),
                "bottom_left" | "bottom-left" => Self::new(false, true, true, false),
                "bottom_right" | "bottom-right" => Self::new(false, true, false, true),
                _ => Self::default(),
            },
        }
    }
}

impl PanelAnchor {
    /// Creates a new `PanelAnchor` with explicit edge flags.
    ///
    /// Multiple edges can be enabled simultaneously to control panel placement and stretching.
    #[allow(clippy::fn_params_excessive_bools)]
    #[must_use]
    pub const fn new(top: bool, bottom: bool, left: bool, right: bool) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }

    #[must_use]
    pub const fn none() -> Self {
        Self {
            top: false,
            bottom: false,
            left: false,
            right: false,
        }
    }

    #[must_use]
    pub const fn all() -> Self {
        Self {
            top: true,
            bottom: true,
            left: true,
            right: true,
        }
    }

    #[must_use]
    pub const fn top(&self) -> bool {
        self.top
    }

    #[must_use]
    pub const fn bottom(&self) -> bool {
        self.bottom
    }

    #[must_use]
    pub const fn left(&self) -> bool {
        self.left
    }

    #[must_use]
    pub const fn right(&self) -> bool {
        self.right
    }
}
