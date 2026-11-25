//! `FSPX`: FidelityFX (`amd_fidelityfx_upscaler_xbox.dll`) `ACCC` compute shader container.
//!
//! Pure parser adapted from Leonida (`docs/fspx.md`, GPL-3.0-or-later).

use std::fmt;

pub const MAGIC: &[u8; 4] = b"FSPX";
pub const HEADER_SIZE: usize = 16;
pub const ENTRY_SIZE: usize = 16;

pub const KIND_DISPATCH_STATE: u32 = 0;
pub const KIND_EXTRA: u32 = 1;
pub const KIND_PROGRAM: u32 = 6;
pub const KIND_RESOURCES: u32 = 7;

/// 7-word dispatch state from `FSPX` kind `0` (28 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchState {
    pub raw: [u32; 7],
    pub num_threads: [u16; 3],
    pub wave32: bool,
    pub compute_pgm_rsrc1: u32,
    pub compute_pgm_rsrc2: u32,
}

impl DispatchState {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 28 {
            return None;
        }
        let mut raw = [0u32; 7];
        for (i, w) in raw.iter_mut().enumerate() {
            *w = u32::from_le_bytes(data[i * 4..i * 4 + 4].try_into().ok()?);
        }
        Some(Self {
            raw,
            num_threads: [
                (raw[3] & 0xFFFF) as u16,
                (raw[3] >> 16) as u16,
                (raw[4] & 0xFFFF) as u16,
            ],
            wave32: (raw[4] & (1 << 20)) != 0,
            compute_pgm_rsrc1: raw[5],
            compute_pgm_rsrc2: raw[6],
        })
    }
}

/// Parsed `FSPX` payload slices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fspx<'a> {
    pub version: u32,
    pub dispatch: &'a [u8],
    pub extra: Option<&'a [u8]>,
    pub program: &'a [u8],
    pub resources: Option<&'a [u8]>,
}

impl<'a> Fspx<'a> {
    /// Decodes the kind-0 dispatch state words.
    pub fn dispatch_state(&self) -> Option<DispatchState> {
        DispatchState::parse(self.dispatch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    TooShort,
    BadMagic,
    DirectoryOutOfBounds,
    MissingRequiredSection,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => write!(f, "FSPX payload shorter than 16 bytes"),
            Self::BadMagic => write!(f, "missing FSPX magic"),
            Self::DirectoryOutOfBounds => write!(f, "FSPX directory exceeds payload size"),
            Self::MissingRequiredSection => write!(f, "FSPX missing required kind 0 or kind 6"),
        }
    }
}

impl std::error::Error for Error {}

/// Parses an `FSPX` payload (such as the `ACCC` part of a FidelityFX DXBC container).
pub fn parse(data: &[u8]) -> Result<Fspx<'_>, Error> {
    if data.len() < HEADER_SIZE {
        return Err(Error::TooShort);
    }
    if data.get(..4) != Some(MAGIC.as_slice()) {
        return Err(Error::BadMagic);
    }
    let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
    let count = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    let total = (u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize).min(data.len());

    let dir_end = count
        .checked_mul(ENTRY_SIZE)
        .and_then(|n| n.checked_add(HEADER_SIZE))
        .ok_or(Error::DirectoryOutOfBounds)?;
    if dir_end > total {
        return Err(Error::DirectoryOutOfBounds);
    }

    let mut dispatch: Option<&[u8]> = None;
    let mut extra: Option<&[u8]> = None;
    let mut program: Option<&[u8]> = None;
    let mut resources: Option<&[u8]> = None;

    for i in 0..count {
        let base = HEADER_SIZE + i * ENTRY_SIZE;
        let kind = u32::from_le_bytes(data[base..base + 4].try_into().unwrap());
        let offset = u32::from_le_bytes(data[base + 4..base + 8].try_into().unwrap()) as usize;
        let size = u32::from_le_bytes(data[base + 8..base + 12].try_into().unwrap()) as usize;
        if offset < dir_end || offset.checked_add(size).is_none_or(|end| end > total) {
            continue;
        }
        let slice = &data[offset..offset + size];
        match kind {
            KIND_DISPATCH_STATE => dispatch = Some(slice),
            KIND_EXTRA => extra = Some(slice),
            KIND_PROGRAM => program = Some(slice),
            KIND_RESOURCES => resources = Some(slice),
            _ => {}
        }
    }

    let dispatch = dispatch.filter(|s| !s.is_empty()).ok_or(Error::MissingRequiredSection)?;
    let program = program.filter(|s| !s.is_empty()).ok_or(Error::MissingRequiredSection)?;

    Ok(Fspx {
        version,
        dispatch,
        extra,
        program,
        resources,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fspx_and_dispatch_state() {
        // Header (16) + 2 entries (32) = 48 bytes; dispatch (28) at 48, program (16) at 76.
        let mut buf = vec![0u8; 92];
        buf[0..4].copy_from_slice(MAGIC);
        buf[4..8].copy_from_slice(&1u32.to_le_bytes());
        buf[8..12].copy_from_slice(&2u32.to_le_bytes());
        let total = buf.len() as u32;
        buf[12..16].copy_from_slice(&total.to_le_bytes());

        // Entry 0: kind 0, offset 48, size 28.
        buf[16..20].copy_from_slice(&KIND_DISPATCH_STATE.to_le_bytes());
        buf[20..24].copy_from_slice(&48u32.to_le_bytes());
        buf[24..28].copy_from_slice(&28u32.to_le_bytes());

        // Entry 1: kind 6, offset 76, size 16.
        buf[32..36].copy_from_slice(&KIND_PROGRAM.to_le_bytes());
        buf[36..40].copy_from_slice(&76u32.to_le_bytes());
        buf[40..44].copy_from_slice(&16u32.to_le_bytes());

        // Dispatch words: 8x8x1 threads, wave32 bit (1 << 20), rsrc1 = 0x11, rsrc2 = 0x22.
        buf[48 + 12..48 + 16].copy_from_slice(&(8u32 | (8u32 << 16)).to_le_bytes());
        buf[48 + 16..48 + 20].copy_from_slice(&(1u32 | (1u32 << 20)).to_le_bytes());
        buf[48 + 20..48 + 24].copy_from_slice(&0x11u32.to_le_bytes());
        buf[48 + 24..48 + 28].copy_from_slice(&0x22u32.to_le_bytes());
        buf[76..92].fill(0xAA);

        let parsed = parse(&buf).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.program, &[0xAA; 16]);
        assert!(parsed.extra.is_none());

        let state = parsed.dispatch_state().unwrap();
        assert_eq!(state.num_threads, [8, 8, 1]);
        assert!(state.wave32);
        assert_eq!(state.compute_pgm_rsrc1, 0x11);
        assert_eq!(state.compute_pgm_rsrc2, 0x22);
    }

    #[test]
    fn rejects_missing_required_kind() {
        let mut buf = vec![0u8; 48];
        buf[0..4].copy_from_slice(MAGIC);
        buf[4..8].copy_from_slice(&1u32.to_le_bytes());
        buf[8..12].copy_from_slice(&1u32.to_le_bytes());
        buf[12..16].copy_from_slice(&48u32.to_le_bytes());
        buf[16..20].copy_from_slice(&KIND_DISPATCH_STATE.to_le_bytes());
        buf[20..24].copy_from_slice(&32u32.to_le_bytes());
        buf[24..28].copy_from_slice(&16u32.to_le_bytes());
        assert_eq!(parse(&buf), Err(Error::MissingRequiredSection));
    }
}
