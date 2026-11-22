//! Pure compositing of embedded child modules into a popup's own pixel buffer.

mod blend;
pub mod layers;
pub(super) mod route;

pub(super) use blend::PixelsMut;
pub(super) use layers::CompositeLayers;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::primitives::render::RenderBuffer;
    use crate::shared::primitives::{ModuleId, MonitorId, Position, Size};

    fn layers_with(origin: Position, buffer: RenderBuffer) -> CompositeLayers {
        let mut layers = CompositeLayers::default();
        layers.upsert(ModuleId::new(1), MonitorId::new("m"), origin, buffer);
        layers
    }

    fn opaque(width: u32, height: u32, px: [u8; 4]) -> RenderBuffer {
        let count = usize::try_from(width.saturating_mul(height)).unwrap_or(0);
        RenderBuffer::new(px.repeat(count), Size::new(width, height))
    }

    #[test]
    fn empty_layers_copy_base_exactly() {
        let base: Vec<u8> = (0..16).collect();
        let mut dst = vec![0u8; 16];
        let mut pixels = PixelsMut::new(&mut dst, Size::new(2, 2));
        pixels.copy_from(&base);
        pixels.blend_layers(&CompositeLayers::default(), 1);
        assert_eq!(dst, base);
    }

    #[test]
    fn short_base_does_not_panic() {
        let base = vec![9u8; 6];
        let mut dst = vec![0u8; 16];
        let layers = layers_with(Position::new(0, 0), opaque(1, 1, [1, 2, 3, 255]));
        let mut pixels = PixelsMut::new(&mut dst, Size::new(2, 2));
        pixels.copy_from(&base);
        pixels.blend_layers(&layers, 1);
        assert_eq!(&dst[..4], &[1, 2, 3, 255]);
        assert_eq!(&dst[4..6], &[9, 9]);
        assert_eq!(&dst[6..], &[0u8; 10]);
    }

    #[test]
    fn scale_two_multiplies_origin() {
        let base = vec![0u8; 4 * 4 * 4];
        let mut dst = vec![0u8; 4 * 4 * 4];
        let layers = layers_with(Position::new(1, 0), opaque(1, 1, [7, 7, 7, 255]));
        let mut pixels = PixelsMut::new(&mut dst, Size::new(4, 4));
        pixels.copy_from(&base);
        pixels.blend_layers(&layers, 2);
        assert_eq!(dst.get(8..12), Some(&[7u8, 7, 7, 255][..]));
        assert_eq!(dst.get(4..8), Some(&[0u8; 4][..]));
    }

    #[test]
    fn opaque_child_replaces_pixels() {
        let base = vec![5u8; 16];
        let mut dst = vec![0u8; 16];
        let layers = layers_with(Position::new(1, 1), opaque(1, 1, [1, 2, 3, 255]));
        let mut pixels = PixelsMut::new(&mut dst, Size::new(2, 2));
        pixels.copy_from(&base);
        pixels.blend_layers(&layers, 1);
        assert_eq!(dst.get(12..16), Some(&[1u8, 2, 3, 255][..]));
        assert_eq!(dst.get(0..12), Some(&[5u8; 12][..]));
    }
}
