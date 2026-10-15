use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{ModuleId, MonitorId, Position};

/// One embedded module's rendered pixels, positioned inside the popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildLayer {
    module_id: ModuleId,
    monitor_id: MonitorId,
    origin: Position,
    buffer: RenderBuffer,
}

impl ChildLayer {
    #[must_use]
    pub const fn module_id(&self) -> ModuleId {
        self.module_id
    }

    #[must_use]
    pub const fn monitor_id(&self) -> &MonitorId {
        &self.monitor_id
    }

    #[must_use]
    pub const fn origin(&self) -> &Position {
        &self.origin
    }

    #[must_use]
    pub const fn buffer(&self) -> &RenderBuffer {
        &self.buffer
    }
}

/// Ordered set of child layers, bottom to top.
#[derive(Debug, Default)]
pub struct CompositeLayers {
    layers: Vec<ChildLayer>,
}

impl CompositeLayers {
    /// Replaces the module's layer in place (keeping its z-position) or adds it on top.
    pub fn upsert(
        &mut self,
        module_id: ModuleId,
        monitor_id: MonitorId,
        origin: Position,
        buffer: RenderBuffer,
    ) {
        let layer = ChildLayer {
            module_id,
            monitor_id,
            origin,
            buffer,
        };
        match self.layers.iter_mut().find(|l| l.module_id == module_id) {
            Some(slot) => *slot = layer,
            None => self.layers.push(layer),
        }
    }

    pub fn remove(&mut self, module_id: ModuleId) {
        self.layers.retain(|l| l.module_id != module_id);
    }

    #[must_use]
    pub fn get(&self, module_id: ModuleId) -> Option<&ChildLayer> {
        self.layers.iter().find(|l| l.module_id == module_id)
    }

    /// Layers from bottom to top.
    pub fn iter(&self) -> impl Iterator<Item = &ChildLayer> {
        self.layers.iter()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::primitives::Size;

    fn buf() -> RenderBuffer {
        RenderBuffer::new(vec![0u8; 4], Size::new(1, 1))
    }

    fn add(layers: &mut CompositeLayers, id: u32, x: i32) {
        layers.upsert(
            ModuleId::new(id),
            MonitorId::new("m"),
            Position::new(x, 0),
            buf(),
        );
    }

    fn order(layers: &CompositeLayers) -> Vec<u32> {
        layers.iter().map(|l| l.module_id().value()).collect()
    }

    #[test]
    fn upsert_replacing_keeps_z_order() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, 0);
        add(&mut layers, 2, 0);
        add(&mut layers, 1, 5);
        assert_eq!(order(&layers), vec![1, 2]);
        assert_eq!(
            layers.get(ModuleId::new(1)).map(|l| l.origin().x()),
            Some(5)
        );
    }

    #[test]
    fn new_layers_go_on_top() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 2, 0);
        add(&mut layers, 1, 0);
        assert_eq!(order(&layers), vec![2, 1]);
    }

    #[test]
    fn remove_drops_layer() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, 0);
        add(&mut layers, 2, 0);
        layers.remove(ModuleId::new(1));
        assert_eq!(order(&layers), vec![2]);
        assert!(layers.get(ModuleId::new(1)).is_none());
        layers.remove(ModuleId::new(2));
        assert!(layers.is_empty());
    }
}
