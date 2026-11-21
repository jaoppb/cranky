use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{Position, Size};

const BPP: i64 = 4;

/// Byte offset of pixel (`x`, `y`) in a row-major buffer `width` pixels wide.
fn offset(x: i64, y: i64, width: i64) -> Option<usize> {
    let idx = y.checked_mul(width)?.checked_add(x)?.checked_mul(BPP)?;
    usize::try_from(idx).ok()
}

/// Premultiplied source-over of `src` onto `dst` at `origin_px`, clipped to `dst_size`.
pub(super) fn blend_over(dst: &mut [u8], dst_size: Size, src: &RenderBuffer, origin_px: Position) {
    let (dst_width, dst_height) = (i64::from(dst_size.width()), i64::from(dst_size.height()));
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
            let dst_pixel = dst.get_mut(dst_offset..dst_offset.saturating_add(4));
            if let (Some(src_pixel), Some(dst_pixel)) = (src_pixel, dst_pixel) {
                blend_pixel(dst_pixel, src_pixel);
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

    fn px(dst: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
        let index = y.saturating_mul(width).saturating_add(x).saturating_mul(4);
        dst.get(index..index.saturating_add(4))
            .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
            .unwrap_or([0; 4])
    }

    #[test]
    fn transparent_child_leaves_base() {
        let mut dst = vec![50u8; 16];
        blend_over(
            &mut dst,
            Size::new(2, 2),
            &buf(2, 2, [0; 4]),
            Position::new(0, 0),
        );
        assert_eq!(dst, vec![50u8; 16]);
    }

    #[test]
    fn half_alpha_premultiplied_blends() {
        let mut dst = vec![255u8; 4];
        blend_over(
            &mut dst,
            Size::new(1, 1),
            &buf(1, 1, [0, 0, 128, 128]),
            Position::new(0, 0),
        );
        // out = src + dst * (255 - 128) / 255 = dst*127/255 = 127 (floor) for 255.
        assert_eq!(dst, vec![127, 127, 128 + 127, 128 + 127]);
    }

    #[test]
    fn child_clipped_at_right_and_bottom_edges() {
        let mut dst = vec![0u8; 16];
        blend_over(
            &mut dst,
            Size::new(2, 2),
            &buf(2, 2, [1, 1, 1, 255]),
            Position::new(1, 1),
        );
        assert_eq!(px(&dst, 2, 1, 1), [1, 1, 1, 255]);
        assert_eq!(px(&dst, 2, 0, 0), [0; 4]);
        assert_eq!(px(&dst, 2, 1, 0), [0; 4]);
        assert_eq!(px(&dst, 2, 0, 1), [0; 4]);
    }

    #[test]
    fn negative_origin_is_clipped() {
        let mut dst = vec![0u8; 16];
        let mut src = vec![0u8; 16];
        src[12..16].copy_from_slice(&[9, 9, 9, 255]);
        blend_over(
            &mut dst,
            Size::new(2, 2),
            &RenderBuffer::new(src, Size::new(2, 2)),
            Position::new(-1, -1),
        );
        assert_eq!(px(&dst, 2, 0, 0), [9, 9, 9, 255]);
        assert_eq!(px(&dst, 2, 1, 0), [0; 4]);
        assert_eq!(px(&dst, 2, 1, 1), [0; 4]);
    }

    #[test]
    fn child_larger_than_dst_does_not_panic() {
        let mut dst = vec![0u8; 16];
        blend_over(
            &mut dst,
            Size::new(2, 2),
            &buf(10, 10, [3, 3, 3, 255]),
            Position::new(0, 0),
        );
        assert_eq!(dst, [3, 3, 3, 255].repeat(4));
    }
}
