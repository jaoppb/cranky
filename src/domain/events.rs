#[derive(Debug, Clone, PartialEq)]
pub enum PointerEvent {
    PointerEnter,
    PointerLeave,
    PointerMotion { x: f64, y: f64 },
    Click { button: u32, x: f64, y: f64 },
    Scroll { axis: u32, amount: f64 },
}


pub type PointerSender = tokio::sync::broadcast::Sender<(crate::domain::ModuleId, PointerEvent)>;
pub type PointerReceiver = tokio::sync::broadcast::Receiver<(crate::domain::ModuleId, PointerEvent)>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pointer_event_variants() {
        let _ = PointerEvent::PointerEnter;
        let _ = PointerEvent::PointerLeave;
        let _ = PointerEvent::PointerMotion { x: 0.0, y: 0.0 };
        let _ = PointerEvent::Click {
            button: 0,
            x: 0.0,
            y: 0.0,
        };
        let _ = PointerEvent::Scroll {
            axis: 0,
            amount: 0.0,
        };
    }
}
