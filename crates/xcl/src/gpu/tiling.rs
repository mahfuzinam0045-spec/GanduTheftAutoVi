//! Re-exported from `victory-formats::tiling` with `xcl::gpu::desc::Image`
//! convenience helpers.
pub use victory_formats::tiling::*;

/// Layout of mip 0 of a T#'s image plus its size in elements. The swizzle the base address
/// carries below the block size becomes `base_xor`; the data starts at
/// `address & !(block_bytes - 1)`.
pub fn image_layout(t: &super::desc::Image, cfg: PipeConfig) -> Option<(Layout, usize, usize)> {
    let (e, side) = element(t.format)?;
    let mut l = Layout::new(t.swizzle_mode, e, cfg)?;
    l.base_xor = (t.address as usize) & (l.block_bytes - 1);
    Some((l, t.width.div_ceil(side) as usize, t.height.div_ceil(side) as usize))
}

/// The mip chain of a T#'s image (all `max_mip + 1` levels of the resource, base swizzle in
/// `base_xor`); the data starts at `address & !(block_bytes - 1)`.
pub fn image_chain(t: &super::desc::Image, cfg: PipeConfig) -> Option<Chain> {
    let (e, side) = element(t.format)?;
    let d3 = matches!(t.ty, super::desc::ImageType::D3);
    let mut l = if d3 {
        Layout::new_3d(t.swizzle_mode, e, cfg)?
    } else {
        Layout::new(t.swizzle_mode, e, cfg)?
    };
    l.base_xor = (t.address as usize) & (l.block_bytes - 1);
    let depth = if d3 { t.depth.max(1) } else { 1 };
    Some(Chain::new_3d(l, t.width, t.height, depth, side, t.max_mip + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_sharp_bc7_layout() {
        let t = crate::gpu::desc::Image {
            address: 0x1_2345_0000 | 0x300,
            format: 181,
            width: 256,
            height: 130,
            depth: 1,
            ty: crate::gpu::desc::ImageType::D2,
            base_level: 0,
            last_level: 0,
            max_mip: 0,
            base_array: 0,
            dst_sel: [4, 5, 6, 7],
            swizzle_mode: 10,
            min_lod: 0.0,
        };
        let (l, w, h) = image_layout(&t, PipeConfig { pipes_log2: 0, pkrs_log2: 0 }).unwrap();
        assert_eq!((w, h, l.block_w, l.block_h, l.base_xor), (64, 33, 64, 64, 0x300));
        assert_eq!(l.size(w, h), 65536);
        assert_eq!(element(56), Some((2, 1)));
        assert_eq!(element(72), None);
    }
}
