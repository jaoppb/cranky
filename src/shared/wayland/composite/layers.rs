//! The embedded child modules' buffers a popup composites: one layer per module, kept in z-order
//! from bottom to top.
use crate::shared::primitives::ModuleId;
use crate::shared::primitives::render::ModuleFrame;

/// Ordered set of child layers, bottom to top: each embedded module's frame, positioned inside the
/// popup.
#[derive(Debug, Default)]
pub(crate) struct CompositeLayers {
    layers: Vec<ModuleFrame>,
}

impl CompositeLayers {
    /// Replaces the module's layer in place (keeping its z-position) or adds it on top.
    pub(crate) fn upsert(&mut self, layer: ModuleFrame) {
        match self
            .layers
            .iter_mut()
            .find(|existing| existing.module_id() == layer.module_id())
        {
            Some(slot) => *slot = layer,
            None => self.layers.push(layer),
        }
    }

    #[cfg(test)]
    pub(crate) fn remove(&mut self, module_id: ModuleId) {
        self.layers.retain(|layer| layer.module_id() != module_id);
    }

    #[must_use]
    pub(crate) fn get(&self, module_id: ModuleId) -> Option<&ModuleFrame> {
        self.layers
            .iter()
            .find(|layer| layer.module_id() == module_id)
    }

    /// Layers from bottom to top.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &ModuleFrame> {
        self.layers.iter()
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::primitives::render::{PlacedBuffer, RenderBuffer};
    use crate::shared::primitives::{MonitorId, Position, Size};

    fn buf() -> RenderBuffer {
        RenderBuffer::new(vec![0u8; 4], Size::new(1, 1))
    }

    fn add(layers: &mut CompositeLayers, id: u32, x: i32) {
        layers.upsert(ModuleFrame::new(
            ModuleId::new(id),
            MonitorId::new("m"),
            PlacedBuffer::new(Position::new(x, 0), buf()),
        ));
    }

    fn order(layers: &CompositeLayers) -> Vec<u32> {
        layers
            .iter()
            .map(|layer| layer.module_id().value())
            .collect()
    }

    #[test]
    fn upsert_replacing_keeps_z_order() {
        let mut layers = CompositeLayers::default();
        add(&mut layers, 1, 0);
        add(&mut layers, 2, 0);
        add(&mut layers, 1, 5);
        assert_eq!(order(&layers), vec![1, 2]);
        assert_eq!(
            layers
                .get(ModuleId::new(1))
                .map(|layer| layer.position().x()),
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
