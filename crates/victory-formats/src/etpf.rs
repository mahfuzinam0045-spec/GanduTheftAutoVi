//! `ETPF` / `RDS5` texture-pool descriptor parser (2D, 2D array, cubemap, 3D volume).
//!
//! Pure parser adapted from Leonida (`docs/textures_arrays.md`, `tools/tex_headers.py`,
//! GPL-3.0-or-later).

/// Texture dimension encoded in word `b` bits 21..=22 of an ETPF descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    Tex2DOrArray,
    Cube,
    Volume3D,
    Unknown(u32),
}

impl Dimension {
    pub fn from_raw(raw: u32) -> Self {
        match raw & 3 {
            1 => Self::Tex2DOrArray,
            2 => Self::Cube,
            3 => Self::Volume3D,
            other => Self::Unknown(other),
        }
    }
}

/// Decoded 16-byte `ETPF` (`RDS5`) texture descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EtpfDescriptor {
    pub width: u32,
    pub height: u32,
    pub packed_mips: u32,
    pub non_tail: u32,
    pub mips: u32,
    /// 7-bit DXGI format ID (`0..=127`).
    pub format: u32,
    /// 11-bit slice count, face count (`6`), or 3D volume depth.
    pub depth: u32,
    pub dimension: Dimension,
    /// Raw 9-bit mode field (`(mode >> 1) & 0xF` is the GFX10 tile mode).
    pub mode: u32,
    pub base_offset: u64,
    pub meta: u32,
    pub extra: u32,
}

impl EtpfDescriptor {
    /// Parses one 16-byte little-endian `ETPF` texture record.
    pub fn parse(record: &[u8]) -> Option<Self> {
        if record.len() < 16 {
            return None;
        }
        let a = u32::from_le_bytes(record[0..4].try_into().ok()?);
        let b = u32::from_le_bytes(record[4..8].try_into().ok()?);
        let c = u32::from_le_bytes(record[8..12].try_into().ok()?);
        let d = u32::from_le_bytes(record[12..16].try_into().ok()?);

        let packed_mips = a >> 28;
        let non_tail = b & 7;
        Some(Self {
            width: (a & 0x3FFF) + 1,
            height: ((a >> 14) & 0x3FFF) + 1,
            packed_mips,
            non_tail,
            mips: packed_mips + non_tail,
            format: (b >> 3) & 0x7F,
            depth: ((b >> 10) & 0x7FF) + 1,
            dimension: Dimension::from_raw((b >> 21) & 3),
            mode: b >> 23,
            base_offset: u64::from(c & 0x1F_FFFF) * 4096,
            meta: c >> 21,
            extra: d,
        })
    }

    /// 4-bit GFX10 tile/swizzle mode (`(mode >> 1) & 0xF`).
    pub fn gfx10_tile_mode(&self) -> u32 {
        (self.mode >> 1) & 0xF
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_cubemap_and_two_slice_array_without_format_aliasing() {
        // 256x256 cubemap: depth = 6 (depth - 1 = 5), format = 71 (BC1_UNORM), dim = 2, mode = 20.
        let a = 255u32 | (255u32 << 14) | (4u32 << 28);
        let b = 2u32 | (71u32 << 3) | (5u32 << 10) | (2u32 << 21) | (20u32 << 23);
        let c = 3u32 | (0x12u32 << 21);
        let d = 0xDEAD_BEEFu32;

        let mut raw = [0u8; 16];
        raw[0..4].copy_from_slice(&a.to_le_bytes());
        raw[4..8].copy_from_slice(&b.to_le_bytes());
        raw[8..12].copy_from_slice(&c.to_le_bytes());
        raw[12..16].copy_from_slice(&d.to_le_bytes());

        let desc = EtpfDescriptor::parse(&raw).unwrap();
        assert_eq!(desc.width, 256);
        assert_eq!(desc.height, 256);
        assert_eq!(desc.mips, 6);
        assert_eq!(desc.format, 71);
        assert_eq!(desc.depth, 6);
        assert_eq!(desc.dimension, Dimension::Cube);
        assert_eq!(desc.gfx10_tile_mode(), 10);
        assert_eq!(desc.base_offset, 3 * 4096);
        assert_eq!(desc.meta, 0x12);
        assert_eq!(desc.extra, d);

        // 2-slice 2D array (depth = 2, depth - 1 = 1): format must remain 28, not 28 | 128.
        let b_arr = 1u32 | (28u32 << 3) | (1u32 << 10) | (1u32 << 21) | (1u32 << 23);
        raw[4..8].copy_from_slice(&b_arr.to_le_bytes());
        let arr = EtpfDescriptor::parse(&raw).unwrap();
        assert_eq!(arr.format, 28);
        assert_eq!(arr.depth, 2);
        assert_eq!(arr.dimension, Dimension::Tex2DOrArray);
    }
}
