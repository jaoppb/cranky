#[derive(Debug, Clone, PartialEq)]
pub enum PointerEvent {
    PointerEnter,
    PointerLeave,
    PointerMotion { x: f64, y: f64 },
    Click { button: u32, x: f64, y: f64 },
    Scroll { axis: u32, amount: f64 },
}
