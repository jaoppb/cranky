use super::CompositeLayers;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{Position, Size};

const BPP: i64 = 4;

/// Byte offset of pixel (`x`, `y`) in a row-major buffer `width` pixels wide.
fn offset(x: i64, y: i64, width: i64) -> Option<usize> {
    let idx = y.checked_mul(width)?.checked_add(x)?.checked_mul(BPP)?;
    usize::try_from(idx).ok()
}

/// A premultiplied ARGB8888 pixel buffer borrowed for writing (an shm slot, in practice), with
/// the size its rows are laid out in.
#[derive(Debug)]
pub(in crate::shared::wayland) struct PixelsMut<'a> {
    data: &'a mut [u8],
    size: Size,
}

impl<'a> PixelsMut<'a> {
    /// Borrows `data` for writing as pixels in rows `size` wide, `size` being the size `data` was
    /// allocated for: blends address pixels through it, and skip any that fall outside `data`.
    pub(in crate::shared::wayland) const fn new(data: &'a mut [u8], size: Size) -> Self {
        Self { data, size }
    }

    /// Copies `base` over these pixels, as far as both reach.
    pub(in crate::shared::wayland) fn copy_from(&mut self, base: &[u8]) {
        let count = self.data.len().min(base.len());
        if let (Some(dst_bytes), Some(base_bytes)) = (self.data.get_mut(..count), base.get(..count))
        {
            dst_bytes.copy_from_slice(base_bytes);
        }
    }

    /// Blends every layer over these pixels, bottom to top. Layer origins are logical and are
    /// multiplied by `scale` to physical pixels.
    pub(in crate::shared::wayland) fn blend_layers(
        &mut self,
        layers: &CompositeLayers,
        scale: i32,
    ) {
        for layer in layers.iter() {
            let origin = Position::new(
                layer.position().x().saturating_mul(scale),
                layer.position().y().saturating_mul(scale),
            );
            self.blend_over(layer.buffer(), origin);
        }
    }

    /// Premultiplied source-over of `src` at `origin_px`, clipped to these pixels.
    fn blend_over(&mut self, src: &RenderBuffer, origin_px: Position) {
        let (dst_width, dst_height) = (i64::from(self.size.width()), i64::from(self.size.height()));
        let (src_width, src_height) = (i64::from(src.width()), i64::from(src.height()));
        let (left, top) = (i64::from(origin_px.x()), i64::from(origin_px.y()));

        let x_start = left.saturating_neg().max(0);
        let x_end = src_width.min(dst_width.saturating_sub(left));
        let y_start = top.saturating_neg().max(0);
        let y_end = src_height.min(dst_height.saturating_sub(top));

        for src_y in y_start..y_end {
            for src_x in x_start..x_end {
                let dst_offset = offset(
                    src_x.saturating_add(left),
                    src_y.saturating_add(top),
                    dst_width,
                );
                let src_offset = offset(src_x, src_y, src_width);
                let (Some(dst_offset), Some(src_offset)) = (dst_offset, src_offset) else {
                    continue;
                };
                let src_pixel = src.data().get(src_offset..src_offset.saturating_add(4));
                let dst_pixel = self.data.get_mut(dst_offset..dst_offset.saturating_add(4));
                if let (Some(src_pixel), Some(dst_pixel)) = (src_pixel, dst_pixel) {
                    blend_pixel(dst_pixel, src_pixel);
                }
            }
        }
    }
}

/// `out = src + dst * (255 - src_alpha) / 255` for all four (premultiplied) channels.
fn blend_pixel(dst: &mut [u8], src: &[u8]) {
    let inv = 255_u32.saturating_sub(src.get(3).copied().map_or(0, u32::from));
    for (dst_channel, src_channel) in dst.iter_mut().zip(src) {
        let scaled = u32::from(*dst_channel).saturating_mul(inv) / 255;
        let out = u32::from(*src_channel).saturating_add(scaled).min(255);
        *dst_channel = u8::try_from(out).unwrap_or(u8::MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(width: u32, height: u32, px: [u8; 4]) -> RenderBuffer {
        let count = usize::try_from(width.saturating_mul(height)).unwrap_or(0);
        RenderBuffer::new(px.repeat(count), Size::new(width, height))
    }

    /// Width of every destination in these tests.
    const WIDTH: usize = 2;

    /// Pixel (`x`, `y`) of a `WIDTH`-wide `dst`.
    fn px(dst: &[u8], x: usize, y: usize) -> [u8; 4] {
        let index = y.saturating_mul(WIDTH).saturating_add(x).saturating_mul(4);
        dst.get(index..index.saturating_add(4))
            .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
            .unwrap_or([0; 4])
    }

    #[test]
    fn transparent_child_leaves_base() {
        let mut dst = vec![50u8; 16];
        PixelsMut::new(&mut dst, Size::new(2, 2))
            .blend_over(&buf(2, 2, [0; 4]), Position::new(0, 0));
        assert_eq!(dst, vec![50u8; 16]);
    }

    #[test]
    fn half_alpha_premultiplied_blends() {
        let mut dst = vec![255u8; 4];
        PixelsMut::new(&mut dst, Size::new(1, 1))
            .blend_over(&buf(1, 1, [0, 0, 128, 128]), Position::new(0, 0));
        // out = src + dst * (255 - 128) / 255 = dst*127/255 = 127 (floor) for 255.
        assert_eq!(dst, vec![127, 127, 128 + 127, 128 + 127]);
    }

    #[test]
    fn child_clipped_at_right_and_bottom_edges() {
        let mut dst = vec![0u8; 16];
        PixelsMut::new(&mut dst, Size::new(2, 2))
            .blend_over(&buf(2, 2, [1, 1, 1, 255]), Position::new(1, 1));
        assert_eq!(px(&dst, 1, 1), [1, 1, 1, 255]);
        assert_eq!(px(&dst, 0, 0), [0; 4]);
        assert_eq!(px(&dst, 1, 0), [0; 4]);
        assert_eq!(px(&dst, 0, 1), [0; 4]);
    }

    #[test]
    fn negative_origin_is_clipped() {
        let mut dst = vec![0u8; 16];
        let mut src = vec![0u8; 16];
        src[12..16].copy_from_slice(&[9, 9, 9, 255]);
        PixelsMut::new(&mut dst, Size::new(2, 2)).blend_over(
            &RenderBuffer::new(src, Size::new(2, 2)),
            Position::new(-1, -1),
        );
        assert_eq!(px(&dst, 0, 0), [9, 9, 9, 255]);
        assert_eq!(px(&dst, 1, 0), [0; 4]);
        assert_eq!(px(&dst, 1, 1), [0; 4]);
    }

    #[test]
    fn child_larger_than_dst_does_not_panic() {
        let mut dst = vec![0u8; 16];
        PixelsMut::new(&mut dst, Size::new(2, 2))
            .blend_over(&buf(10, 10, [3, 3, 3, 255]), Position::new(0, 0));
        assert_eq!(dst, [3, 3, 3, 255].repeat(4));
    }

    #[test]
    fn dst_shorter_than_its_size_blends_only_the_pixels_it_holds() {
        // Two pixels of data laid out as 2x2: the second row lies outside it.
        let mut dst = vec![0u8; 8];
        PixelsMut::new(&mut dst, Size::new(2, 2))
            .blend_over(&buf(2, 2, [4, 4, 4, 255]), Position::new(0, 0));
        assert_eq!(dst, [4, 4, 4, 255].repeat(2));
    }

    #[test]
    fn src_shorter_than_its_size_blends_only_the_pixels_it_holds() {
        let mut dst = vec![0u8; 16];
        PixelsMut::new(&mut dst, Size::new(2, 2)).blend_over(
            &RenderBuffer::new(vec![6, 6, 6, 255], Size::new(2, 2)),
            Position::new(0, 0),
        );
        assert_eq!(px(&dst, 0, 0), [6, 6, 6, 255]);
        assert_eq!(px(&dst, 1, 0), [0; 4]);
        assert_eq!(px(&dst, 0, 1), [0; 4]);
        assert_eq!(px(&dst, 1, 1), [0; 4]);
    }

    #[test]
    fn src_offset_past_i64_skips_the_pixel() {
        // Row 2^31 of a u32::MAX-wide source starts past i64::MAX bytes, and an origin of
        // i32::MIN rows above lands exactly that row on the destination's only pixel.
        let mut dst = vec![7u8; 4];
        PixelsMut::new(&mut dst, Size::new(1, 1)).blend_over(
            &RenderBuffer::new(vec![9, 9, 9, 255], Size::new(u32::MAX, u32::MAX)),
            Position::new(0, i32::MIN),
        );
        assert_eq!(dst, vec![7u8; 4]);
    }
}
