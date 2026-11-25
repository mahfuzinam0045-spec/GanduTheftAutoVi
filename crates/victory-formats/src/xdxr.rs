//! Xbox Series `XDXR` BVH4 ray-tracing acceleration structure header & node pointers.
//!
//! Pure parser/validator adapted from Leonida (`docs/xdxr.md`, `tools/xdxr_bvh.py`,
//! GPL-3.0-or-later).

use std::fmt;

pub const UNIT: usize = 64;
pub const HEADER_SIZE: usize = 64;
pub const NODE16_SIZE: usize = 64;
pub const NODE32_SIZE: usize = 128;
pub const LEAF_SIZE: usize = 64;
pub const INSTANCE_SIZE: usize = 128;
pub const AS_ALIGNMENT: usize = 256;

pub const INVALID_NODE_PTR: u32 = 0xFFFF_FFFF;

pub const FLAG_BOTTOM_LEVEL: u32 = 0x8000_0000;
pub const FLAG_COMPRESSED: u32 = 0x0001_0000;
pub const FLAG_OFFLINE: u32 = 0x0002_0000;
pub const FLAG_UPDATEABLE: u32 = 0x0004_0000;
pub const FLAG_DEHYDRATED: u32 = 0x0100_0000;
pub const VERSION_2403: u32 = 0x0006;

/// BVH4 node kinds encoded in the low 3 bits of a 32-bit node pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeKind {
    Tri0 = 0,
    Tri1 = 1,
    Tri2 = 2,
    Tri3 = 3,
    Box16 = 4,
    Box32 = 5,
    Instance = 6,
    Aabb = 7,
}

impl NodeKind {
    pub fn from_raw(bits: u32) -> Self {
        match bits & 7 {
            0 => Self::Tri0,
            1 => Self::Tri1,
            2 => Self::Tri2,
            3 => Self::Tri3,
            4 => Self::Box16,
            5 => Self::Box32,
            6 => Self::Instance,
            _ => Self::Aabb,
        }
    }

    pub fn byte_size(self) -> usize {
        match self {
            Self::Box32 => NODE32_SIZE,
            Self::Instance => INSTANCE_SIZE,
            _ => UNIT,
        }
    }
}

/// 32-bit BVH4 node pointer (`(byte_offset / 64) << 3 | kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodePtr(pub u32);

impl NodePtr {
    pub fn pack(offset: usize, kind: NodeKind) -> Option<Self> {
        if !offset.is_multiple_of(UNIT) {
            return None;
        }
        let unit_idx = u32::try_from(offset / UNIT).ok()?;
        let raw = unit_idx.checked_shl(3)? | u32::from(kind as u8);
        if raw == INVALID_NODE_PTR {
            return None;
        }
        Some(Self(raw))
    }

    pub fn is_invalid(self) -> bool {
        self.0 == INVALID_NODE_PTR
    }

    pub fn kind(self) -> NodeKind {
        NodeKind::from_raw(self.0)
    }

    pub fn offset(self) -> usize {
        ((self.0 >> 3) as usize) * UNIT
    }
}

/// 64-byte XDXR BVH4 header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BvhHeader {
    pub flags: u32,
    pub total_size_units: u32,
    pub crc: u32,
    pub num_elements: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub offset_to_nodes: i32,
    pub remap: u32,
    pub root_node: NodePtr,
    pub offset_to_elements: u32,
    pub offset_to_parents: i32,
    pub cp: u32,
}

impl BvhHeader {
    pub fn is_bottom_level(&self) -> bool {
        (self.flags & FLAG_BOTTOM_LEVEL) != 0
    }

    pub fn is_compressed_or_dehydrated(&self) -> bool {
        (self.flags & (FLAG_COMPRESSED | FLAG_DEHYDRATED)) != 0
    }

    /// Expected byte length of the BVH4 buffer (`64 * (total_size_units + 1)`).
    pub fn expected_byte_len(&self) -> Option<usize> {
        (self.total_size_units as usize)
            .checked_add(1)?
            .checked_mul(UNIT)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    TooShort,
    CompressedOrDehydrated,
    SizeMismatch { expected: usize, actual: usize },
    UnalignedOrOutOfBoundsOffset(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => write!(f, "buffer shorter than 64-byte BVH4 header"),
            Self::CompressedOrDehydrated => {
                write!(f, "compressed or dehydrated BVH4 (not a runtime tree)")
            }
            Self::SizeMismatch { expected, actual } => {
                write!(f, "BVH4 size mismatch: expected {expected} B, got {actual} B")
            }
            Self::UnalignedOrOutOfBoundsOffset(field) => {
                write!(f, "BVH4 {field} is unaligned or out of bounds")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Parses the 64-byte BVH4 header without validating the full buffer.
pub fn parse_header(data: &[u8]) -> Result<BvhHeader, Error> {
    if data.len() < HEADER_SIZE {
        return Err(Error::TooShort);
    }
    let u32_at = |o: usize| u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
    let i32_at = |o: usize| i32::from_le_bytes(data[o..o + 4].try_into().unwrap());
    let f32_at = |o: usize| f32::from_le_bytes(data[o..o + 4].try_into().unwrap());

    Ok(BvhHeader {
        flags: u32_at(0x00),
        total_size_units: u32_at(0x04),
        crc: u32_at(0x08),
        num_elements: u32_at(0x0C),
        bounds_min: [f32_at(0x10), f32_at(0x14), f32_at(0x18)],
        bounds_max: [f32_at(0x1C), f32_at(0x20), f32_at(0x24)],
        offset_to_nodes: i32_at(0x28),
        remap: u32_at(0x2C),
        root_node: NodePtr(u32_at(0x30)),
        offset_to_elements: u32_at(0x34),
        offset_to_parents: i32_at(0x38),
        cp: u32_at(0x3C),
    })
}

/// Parses and validates a runtime BVH4 buffer's header, size, offsets, and root pointer.
pub fn validate_runtime_bvh(data: &[u8]) -> Result<BvhHeader, Error> {
    let hdr = parse_header(data)?;
    if hdr.is_compressed_or_dehydrated() {
        return Err(Error::CompressedOrDehydrated);
    }
    let expected = hdr
        .expected_byte_len()
        .ok_or(Error::UnalignedOrOutOfBoundsOffset("total_size_units"))?;
    if expected != data.len() {
        return Err(Error::SizeMismatch {
            expected,
            actual: data.len(),
        });
    }
    if hdr.offset_to_nodes < 0
        || !(hdr.offset_to_nodes as usize).is_multiple_of(UNIT)
        || hdr.offset_to_nodes as usize > data.len()
    {
        return Err(Error::UnalignedOrOutOfBoundsOffset("offset_to_nodes"));
    }
    let elem_off = hdr.offset_to_elements as usize;
    if !elem_off.is_multiple_of(UNIT) || elem_off > data.len() {
        return Err(Error::UnalignedOrOutOfBoundsOffset("offset_to_elements"));
    }
    if !hdr.root_node.is_invalid() {
        let off = hdr.root_node.offset();
        let end = off
            .checked_add(hdr.root_node.kind().byte_size())
            .ok_or(Error::UnalignedOrOutOfBoundsOffset("root_node"))?;
        if off < HEADER_SIZE || end > data.len() {
            return Err(Error::UnalignedOrOutOfBoundsOffset("root_node"));
        }
    }
    Ok(hdr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_ptr_roundtrips() {
        let ptr = NodePtr::pack(0x140, NodeKind::Box32).unwrap();
        assert_eq!(ptr.kind(), NodeKind::Box32);
        assert_eq!(ptr.offset(), 0x140);
        assert_eq!(ptr.kind().byte_size(), NODE32_SIZE);
        assert!(NodePtr::pack(0x144, NodeKind::Tri0).is_none());
    }

    #[test]
    fn validates_runtime_blas_header_and_rejects_dehydrated() {
        // 64B header + 64B Box16 root = 128B (total_size_units = 1).
        let mut buf = vec![0u8; 128];
        buf[0x00..0x04].copy_from_slice(&(FLAG_BOTTOM_LEVEL | VERSION_2403).to_le_bytes());
        buf[0x04..0x08].copy_from_slice(&1u32.to_le_bytes());
        buf[0x28..0x2C].copy_from_slice(&64i32.to_le_bytes());
        let root = NodePtr::pack(64, NodeKind::Box16).unwrap();
        buf[0x30..0x34].copy_from_slice(&root.0.to_le_bytes());
        buf[0x34..0x38].copy_from_slice(&128u32.to_le_bytes());

        let hdr = validate_runtime_bvh(&buf).unwrap();
        assert!(hdr.is_bottom_level());
        assert_eq!(hdr.root_node, root);

        // Dehydrated flag must be rejected for runtime BVH validation.
        buf[0x00..0x04]
            .copy_from_slice(&(FLAG_BOTTOM_LEVEL | FLAG_DEHYDRATED | VERSION_2403).to_le_bytes());
        assert_eq!(validate_runtime_bvh(&buf), Err(Error::CompressedOrDehydrated));
    }
}
