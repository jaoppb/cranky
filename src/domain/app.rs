use crate::ports::canvas::Canvas;
use crate::domain::errors::DomainError;

pub struct CrankyApp {
    // This will be implemented in Phase 3
}

impl CrankyApp {
    pub fn new() -> Self {
        Self {}
    }

    /// Renders the current state of a specific output onto the provided canvas.
    /// This method is generic to support static dispatch for the rendering engine.
    pub fn render<C: Canvas>(&self, _output_id: u32, _canvas: &mut C) -> Result<(), DomainError> {
        // Implementation will follow in Phase 3
        Ok(())
    }
}
