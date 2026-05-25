use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    PointerEnter,
    PointerLeave,
    Click { button: u32, x: f64, y: f64 },
    Scroll { axis: u32, amount: f64 },
}
