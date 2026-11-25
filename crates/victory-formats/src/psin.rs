//! `PSIN`: RAGE's metadata container (script/data reflection schemas & instance blocks).
//!
//! Pure parser adapted from Leonida (`docs/psin.md`, GPL-3.0-or-later).

use std::fmt;

pub const MAGIC: &[u8; 4] = b"PSIN";
pub const HEADER_SIZE: usize = 16;

/// One tagged section inside a `PSIN` container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section<'a> {
    pub tag: [u8; 4],
    pub offset: usize,
    pub total_size: usize,
    pub payload: &'a [u8],
}

/// Parsed `PSIN` header and section table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container<'a> {
    pub first_section_offset: usize,
    pub first_block_type: u32,
    pub first_block_size: u32,
    pub instance_data: &'a [u8],
    pub sections: Vec<Section<'a>>,
}

impl<'a> Container<'a> {
    /// Returns the first section matching `tag` (e.g. `b"LSCH"`, `b"LENM"`).
    pub fn section(&self, tag: &[u8; 4]) -> Option<&Section<'a>> {
        self.sections.iter().find(|s| &s.tag == tag)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BadMagic,
    TooShort,
    InvalidSectionOffset(usize),
    InvalidSectionSize { offset: usize, size: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "missing PSIN magic"),
            Self::TooShort => write!(f, "buffer shorter than 16-byte PSIN header"),
            Self::InvalidSectionOffset(off) => {
                write!(f, "PSIN first section offset 0x{off:x} out of range")
            }
            Self::InvalidSectionSize { offset, size } => {
                write!(f, "invalid PSIN section size {size} at 0x{offset:x}")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Parses a complete `PSIN` container and validates every section header.
pub fn parse(data: &[u8]) -> Result<Container<'_>, Error> {
    if data.get(..4) != Some(MAGIC.as_slice()) {
        return Err(Error::BadMagic);
    }
    if data.len() < HEADER_SIZE {
        return Err(Error::TooShort);
    }
    let first_section_offset = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
    let first_block_type = u32::from_be_bytes(data[8..12].try_into().unwrap());
    let first_block_size = u32::from_be_bytes(data[12..16].try_into().unwrap());

    if first_section_offset < HEADER_SIZE || first_section_offset > data.len() {
        return Err(Error::InvalidSectionOffset(first_section_offset));
    }

    let instance_data = &data[8..first_section_offset];
    let mut sections = Vec::new();
    let mut pos = first_section_offset;

    while pos < data.len() {
        if pos + 8 > data.len() {
            return Err(Error::InvalidSectionSize {
                offset: pos,
                size: data.len() - pos,
            });
        }
        let tag: [u8; 4] = data[pos..pos + 4].try_into().unwrap();
        let total_size = u32::from_be_bytes(data[pos + 4..pos + 8].try_into().unwrap()) as usize;
        if total_size < 8 || pos.checked_add(total_size).is_none_or(|end| end > data.len()) {
            return Err(Error::InvalidSectionSize {
                offset: pos,
                size: total_size,
            });
        }
        sections.push(Section {
            tag,
            offset: pos,
            total_size,
            payload: &data[pos + 8..pos + total_size],
        });
        pos += total_size;
    }

    Ok(Container {
        first_section_offset,
        first_block_type,
        first_block_size,
        instance_data,
        sections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_section(out: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
        out.extend_from_slice(tag);
        out.extend_from_slice(&((8 + payload.len()) as u32).to_be_bytes());
        out.extend_from_slice(payload);
    }

    #[test]
    fn parses_header_and_sections() {
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);
        buf.extend_from_slice(&24u32.to_be_bytes()); // first section at +24
        buf.extend_from_slice(&22u32.to_be_bytes()); // block type 22 (0x16)
        buf.extend_from_slice(&8u32.to_be_bytes());
        buf.extend_from_slice(&[0u8; 8]); // remaining instance bytes up to +24

        push_section(&mut buf, b"LSCH", &[1, 2, 3, 4]);
        push_section(&mut buf, b"LGID", &[]);
        push_section(&mut buf, b"CHKS", &[0xAA, 0xBB, 0xCC, 0xDD]);

        let c = parse(&buf).unwrap();
        assert_eq!(c.first_section_offset, 24);
        assert_eq!(c.first_block_type, 22);
        assert_eq!(c.first_block_size, 8);
        assert_eq!(c.instance_data.len(), 16);
        assert_eq!(c.sections.len(), 3);
        assert_eq!(c.section(b"LSCH").unwrap().payload, &[1, 2, 3, 4]);
        assert!(c.section(b"LGID").unwrap().payload.is_empty());
        assert_eq!(c.section(b"CHKS").unwrap().payload, &[0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn rejects_bad_magic_and_truncated_section() {
        assert_eq!(parse(b"NOPE000000000000"), Err(Error::BadMagic));
        let mut bad = Vec::new();
        bad.extend_from_slice(MAGIC);
        bad.extend_from_slice(&16u32.to_be_bytes());
        bad.extend_from_slice(&0u64.to_be_bytes());
        bad.extend_from_slice(b"LSCH");
        bad.extend_from_slice(&64u32.to_be_bytes()); // claims 64 bytes, none follow
        assert_eq!(
            parse(&bad),
            Err(Error::InvalidSectionSize {
                offset: 16,
                size: 64
            })
        );
    }
}
