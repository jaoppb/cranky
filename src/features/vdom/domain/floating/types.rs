use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PanelLayer {
    Background,
    Bottom,
    #[default]
    Top,
    Overlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "ExclusiveZoneHelper")]
pub(crate) struct ExclusiveZone(i32);

#[derive(Deserialize)]
#[serde(untagged)]
enum ExclusiveZoneHelper {
    Number(i32),
    Named(String),
}

impl From<ExclusiveZoneHelper> for ExclusiveZone {
    fn from(helper: ExclusiveZoneHelper) -> Self {
        match helper {
            ExclusiveZoneHelper::Number(value) => Self::new(value),
            ExclusiveZoneHelper::Named(name) => match name.to_lowercase().as_str() {
                "auto" => Self::auto(),
                _ => Self::none(),
            },
        }
    }
}

impl ExclusiveZone {
    #[must_use]
    pub(crate) const fn new(value: i32) -> Self {
        Self(value)
    }

    #[must_use]
    pub(crate) const fn auto() -> Self {
        Self(-1)
    }

    #[must_use]
    pub(crate) const fn none() -> Self {
        Self(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum KeyboardInteractivity {
    #[default]
    None,
    Exclusive,
    OnDemand,
}
