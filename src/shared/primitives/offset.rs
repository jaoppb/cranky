use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "PopupOffsetHelper")]
pub struct PopupOffset {
    dx: i32,
    dy: i32,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PopupOffsetHelper {
    Map {
        #[serde(alias = "x", default)]
        dx: i32,
        #[serde(alias = "y", default)]
        dy: i32,
    },
    Tuple((i32, i32)),
    List(Vec<i32>),
}

impl From<PopupOffsetHelper> for PopupOffset {
    fn from(helper: PopupOffsetHelper) -> Self {
        match helper {
            PopupOffsetHelper::Map { dx, dy } | PopupOffsetHelper::Tuple((dx, dy)) => {
                Self::new(dx, dy)
            }
            PopupOffsetHelper::List(list) => {
                let dx = list.first().copied().unwrap_or(0);
                let dy = list.get(1).copied().unwrap_or(0);
                Self::new(dx, dy)
            }
        }
    }
}

impl PopupOffset {
    #[must_use]
    pub const fn new(dx: i32, dy: i32) -> Self {
        Self { dx, dy }
    }

    #[must_use]
    pub const fn dx(&self) -> i32 {
        self.dx
    }

    #[must_use]
    pub const fn dy(&self) -> i32 {
        self.dy
    }
}
