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
    let (dw, dh) = (i64::from(dst_size.width()), i64::from(dst_size.height()));
    let (sw, sh) = (i64::from(src.width()), i64::from(src.height()));
    let (ox, oy) = (i64::from(origin_px.x()), i64::from(origin_px.y()));

    let x_start = ox.saturating_neg().max(0);
    let x_end = sw.min(dw.saturating_sub(ox));
    let y_start = oy.saturating_neg().max(0);
    let y_end = sh.min(dh.saturating_sub(oy));

    for sy in y_start..y_end {
        for sx in x_start..x_end {
            let di = offset(sx.saturating_add(ox), sy.saturating_add(oy), dw);
            let si = offset(sx, sy, sw);
            let (Some(di), Some(si)) = (di, si) else {
                continue;
            };
            let s = src.data().get(si..si.saturating_add(4));
            let d = dst.get_mut(di..di.saturating_add(4));
            if let (Some(s), Some(d)) = (s, d) {
                blend_pixel(d, s);
            }
        }
    }
}

/// `out = s + d * (255 - s_a) / 255` for all four (premultiplied) channels.
fn blend_pixel(d: &mut [u8], s: &[u8]) {
    let inv = 255_u32.saturating_sub(s.get(3).copied().map_or(0, u32::from));
    for (dc, sc) in d.iter_mut().zip(s) {
        let scaled = u32::from(*dc).saturating_mul(inv) / 255;
        let out = u32::from(*sc).saturating_add(scaled).min(255);
        *dc = u8::try_from(out).unwrap_or(u8::MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(w: u32, h: u32, px: [u8; 4]) -> RenderBuffer {
        let n = usize::try_from(w.saturating_mul(h)).unwrap_or(0);
        RenderBuffer::new(px.repeat(n), Size::new(w, h))
    }

    fn px(dst: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
        let i = y.saturating_mul(width).saturating_add(x).saturating_mul(4);
        dst.get(i..i.saturating_add(4))
            .and_then(|s| <[u8; 4]>::try_from(s).ok())
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
        // out = s + d * (255 - 128) / 255 = d*127/255 = 127 (floor) for 255.
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
