//! `ADAT` (AWC) audio banks and embedded `RSC8` v8001 clip headers.
//!
//! Pure parser adapted from Leonida (`docs/rsc8_audio.md`, `tools/rsc8_audio.py`,
//! GPL-3.0-or-later).

use std::fmt;

pub const ADAT_MAGIC: &[u8; 4] = b"ADAT";
pub const RSC8_MAGIC: &[u8; 4] = b"RSC8";
pub const RSC8_VERSION: u32 = 8001;
pub const RSC8_VERSION_FLAGGED: u32 = 0x0100_1F41;

pub const TAG_PEAK: u8 = 0x36;
pub const TAG_STREAMED_MUSIC: u8 = 0x48;
pub const TAG_DATA: u8 = 0x55;
pub const TAG_GRID20: u8 = 0x5A;
pub const TAG_RSC8_CLIP: u8 = 0x5C;
pub const TAG_FORMAT: u8 = 0x76;
pub const TAG_SEEKTABLE: u8 = 0xA3;
pub const TAG_LOUDNESS: u8 = 0xB2;
pub const TAG_MARKER: u8 = 0xBD;
pub const TAG_INDEXRUN: u8 = 0xD9;

const MAX_HEADER_BYTES: usize = 32 * 1024 * 1024;
const MAX_BANK_STREAMS: usize = 200_000;

/// A single tagged chunk belonging to an AWC stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag {
    pub kind: u8,
    pub offset: usize,
    pub size: usize,
}

/// One stream entry inside an `ADAT` bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    pub id: u32,
    pub tags: Vec<Tag>,
}

/// Parsed `ADAT` (AWC) bank header and stream/tag directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bank {
    pub flags: u32,
    pub header_size: usize,
    pub total_size: usize,
    pub streams: Vec<Stream>,
}

impl Bank {
    /// Returns the payload slice for `tag` inside `bank_bytes`, if in bounds.
    pub fn tag_bytes<'a>(&self, bank_bytes: &'a [u8], tag: &Tag) -> Option<&'a [u8]> {
        let end = tag.offset.checked_add(tag.size)?;
        bank_bytes.get(tag.offset..end)
    }
}

/// Audio stream metadata from a `0x76` format tag (32 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatChunk {
    pub samples: u32,
    pub sample_rate: u16,
    pub codec: u8,
    pub channels: u8,
}

impl FormatChunk {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 32 {
            return None;
        }
        Some(Self {
            samples: u32::from_le_bytes(data[0..4].try_into().ok()?),
            sample_rate: u16::from_le_bytes(data[8..10].try_into().ok()?),
            codec: data[0x1C],
            channels: data[0x1D],
        })
    }
}

/// Header of an `RSC8` v8001 facial/lip-sync animation clip in tag `0x5c`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rsc8Header {
    pub version: u32,
    pub body_size: u32,
    pub flags: u32,
}

impl Rsc8Header {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 16 || data.get(..4) != Some(RSC8_MAGIC.as_slice()) {
            return None;
        }
        let version = u32::from_le_bytes(data[4..8].try_into().ok()?);
        if version != RSC8_VERSION && version != RSC8_VERSION_FLAGGED {
            return None;
        }
        let body_size = u32::from_le_bytes(data[8..12].try_into().ok()?);
        let flags = u32::from_le_bytes(data[12..16].try_into().ok()?);
        if flags != 1 || usize::try_from(body_size).ok()?.checked_add(12)? != data.len() {
            return None;
        }
        Some(Self {
            version,
            body_size,
            flags,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BadMagic,
    InvalidHeader,
    OutOfBounds,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "missing ADAT magic"),
            Self::InvalidHeader => write!(f, "invalid ADAT header"),
            Self::OutOfBounds => write!(f, "ADAT directory or tag out of bounds"),
        }
    }
}

impl std::error::Error for Error {}

/// Parses an `ADAT` (AWC) bank header and validates all tag ranges against `data.len()`.
pub fn parse_bank(data: &[u8]) -> Result<Bank, Error> {
    if data.get(..4) != Some(ADAT_MAGIC.as_slice()) {
        return Err(Error::BadMagic);
    }
    if data.len() < 16 {
        return Err(Error::InvalidHeader);
    }
    let flags = u32::from_le_bytes(data[4..8].try_into().unwrap());
    let count = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    let header_size = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;

    if count > MAX_BANK_STREAMS
        || !(16..=MAX_HEADER_BYTES).contains(&header_size)
        || header_size > data.len()
    {
        return Err(Error::InvalidHeader);
    }

    let mut pos = 16usize;
    if (flags & 0x0040_0000) != 0 {
        pos = pos.checked_add(4).ok_or(Error::InvalidHeader)?;
    }
    if (flags & 0x0001_0000) != 0 {
        pos = pos
            .checked_add(count.checked_mul(2).ok_or(Error::InvalidHeader)?)
            .ok_or(Error::InvalidHeader)?;
    }
    if (flags & 0x0020_0000) != 0 {
        pos = pos
            .checked_add(
                (count + 1)
                    .checked_mul(4)
                    .ok_or(Error::InvalidHeader)?,
            )
            .ok_or(Error::InvalidHeader)?;
    }

    if count == 0 {
        // Zero-stream banks are valid iff the flag-selected tables fill the header exactly.
        if pos != header_size {
            return Err(Error::InvalidHeader);
        }
        return Ok(Bank {
            flags,
            header_size,
            total_size: header_size,
            streams: Vec::new(),
        });
    }

    let stream_table_end = pos
        .checked_add(count.checked_mul(4).ok_or(Error::InvalidHeader)?)
        .ok_or(Error::InvalidHeader)?;
    if stream_table_end > header_size {
        return Err(Error::InvalidHeader);
    }

    let mut stream_specs = Vec::with_capacity(count);
    let mut total_tags = 0usize;
    for _ in 0..count {
        let w = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;
        let tc = (w >> 29) as usize;
        let sid = w & 0x1FFF_FFFF;
        total_tags = total_tags.checked_add(tc).ok_or(Error::InvalidHeader)?;
        stream_specs.push((tc, sid));
    }

    let tag_table_end = pos
        .checked_add(total_tags.checked_mul(8).ok_or(Error::InvalidHeader)?)
        .ok_or(Error::InvalidHeader)?;
    if tag_table_end > header_size {
        return Err(Error::InvalidHeader);
    }

    let mut total_size = header_size;
    let mut streams = Vec::with_capacity(count);
    for (tc, sid) in stream_specs {
        let mut tags = Vec::with_capacity(tc);
        for _ in 0..tc {
            let q = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            pos += 8;
            let kind = ((q >> 56) & 0xFF) as u8;
            let size = ((q >> 28) & 0x0FFF_FFFF) as usize;
            let offset = (q & 0x0FFF_FFFF) as usize;
            let end = offset.checked_add(size).ok_or(Error::OutOfBounds)?;
            if end > data.len() {
                return Err(Error::OutOfBounds);
            }
            if end > total_size {
                total_size = end;
            }
            tags.push(Tag { kind, offset, size });
        }
        streams.push(Stream { id: sid, tags });
    }

    Ok(Bank {
        flags,
        header_size,
        total_size,
        streams,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_zero_stream_bank_and_rejects_slop() {
        let mut bank = Vec::new();
        bank.extend_from_slice(ADAT_MAGIC);
        bank.extend_from_slice(&0xFF21_0001u32.to_le_bytes()); // 0x200000 -> (0+1)*4 = 4 bytes -> 20 total
        bank.extend_from_slice(&0u32.to_le_bytes());
        bank.extend_from_slice(&20u32.to_le_bytes());
        bank.extend_from_slice(&0u32.to_le_bytes());

        let parsed = parse_bank(&bank).unwrap();
        assert_eq!(parsed.header_size, 20);
        assert!(parsed.streams.is_empty());

        // Header slop on count == 0 must be rejected.
        bank[12..16].copy_from_slice(&24u32.to_le_bytes());
        bank.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(parse_bank(&bank), Err(Error::InvalidHeader));
    }

    #[test]
    fn parses_stream_with_format_and_rsc8_clip() {
        // Header: 16 bytes + 1 stream (4 bytes) + 2 tags (16 bytes) = 36 bytes.
        let fmt_offset = 36usize;
        let fmt_size = 32usize;
        let rsc8_offset = fmt_offset + fmt_size;
        let rsc8_size = 20usize;

        let mut buf = vec![0u8; rsc8_offset + rsc8_size];
        buf[0..4].copy_from_slice(ADAT_MAGIC);
        buf[4..8].copy_from_slice(&0u32.to_le_bytes());
        buf[8..12].copy_from_slice(&1u32.to_le_bytes());
        buf[12..16].copy_from_slice(&(fmt_offset as u32).to_le_bytes());

        // Stream 0: tag_count = 2, id = 0x1234.
        let sw = (2u32 << 29) | 0x1234;
        buf[16..20].copy_from_slice(&sw.to_le_bytes());

        let q_fmt = (u64::from(TAG_FORMAT) << 56)
            | ((fmt_size as u64) << 28)
            | (fmt_offset as u64);
        let q_rsc8 = (u64::from(TAG_RSC8_CLIP) << 56)
            | ((rsc8_size as u64) << 28)
            | (rsc8_offset as u64);
        buf[20..28].copy_from_slice(&q_fmt.to_le_bytes());
        buf[28..36].copy_from_slice(&q_rsc8.to_le_bytes());

        // 0x76 format chunk: 48000 Hz, 1 ch, Opus (0x0B), 96000 samples.
        buf[fmt_offset..fmt_offset + 4].copy_from_slice(&96_000u32.to_le_bytes());
        buf[fmt_offset + 8..fmt_offset + 10].copy_from_slice(&48_000u16.to_le_bytes());
        buf[fmt_offset + 0x1C] = 0x0B;
        buf[fmt_offset + 0x1D] = 1;

        // 0x5c RSC8 v8001 header: body_size = rsc8_size - 12 = 8, flags = 1.
        buf[rsc8_offset..rsc8_offset + 4].copy_from_slice(RSC8_MAGIC);
        buf[rsc8_offset + 4..rsc8_offset + 8].copy_from_slice(&RSC8_VERSION.to_le_bytes());
        buf[rsc8_offset + 8..rsc8_offset + 12].copy_from_slice(&8u32.to_le_bytes());
        buf[rsc8_offset + 12..rsc8_offset + 16].copy_from_slice(&1u32.to_le_bytes());

        let bank = parse_bank(&buf).unwrap();
        assert_eq!(bank.streams.len(), 1);
        assert_eq!(bank.streams[0].id, 0x1234);
        assert_eq!(bank.total_size, buf.len());

        let fmt = FormatChunk::parse(bank.tag_bytes(&buf, &bank.streams[0].tags[0]).unwrap()).unwrap();
        assert_eq!(
            fmt,
            FormatChunk {
                samples: 96_000,
                sample_rate: 48_000,
                codec: 0x0B,
                channels: 1,
            }
        );

        let rsc8 = Rsc8Header::parse(bank.tag_bytes(&buf, &bank.streams[0].tags[1]).unwrap()).unwrap();
        assert_eq!(
            rsc8,
            Rsc8Header {
                version: RSC8_VERSION,
                body_size: 8,
                flags: 1,
            }
        );
    }
}
