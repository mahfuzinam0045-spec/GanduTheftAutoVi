//! `bcrypt.dll` RNG and algorithm provider shims.
//!
//! Tracks live `BCRYPT_ALG_HANDLE`s so double-close, null, or foreign handles
//! fail safely with `STATUS_INVALID_HANDLE` instead of faulting in host
//! `bcryptprimitives.dll`, delegates to host `bcrypt.dll`, and supports an
//! optional deterministic `VICTORY_RNG_SEED=<u64>` mode for reproducible runs.
//!
//! Adapted from Leonida (`runtime/shims/bcrypt_rng.cpp`, GPL-3.0-or-later).
#![allow(non_snake_case)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{LazyLock, Mutex};

use windows_sys::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};

use crate::{HleModule, hle_export};

type NtStatus = i32;
type AlgHandle = *mut c_void;

const STATUS_SUCCESS: NtStatus = 0;
const STATUS_UNSUCCESSFUL: NtStatus = 0xC000_0001_u32 as i32;
const STATUS_INVALID_HANDLE: NtStatus = 0xC000_0008_u32 as i32;
const STATUS_INVALID_PARAMETER: NtStatus = 0xC000_000D_u32 as i32;
const STATUS_NOT_FOUND: NtStatus = 0xC000_0225_u32 as i32;

const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x0000_0002;

type OpenAlgFn = unsafe extern "system" fn(*mut AlgHandle, *const u16, *const u16, u32) -> NtStatus;
type CloseAlgFn = unsafe extern "system" fn(AlgHandle, u32) -> NtStatus;
type GenRandomFn = unsafe extern "system" fn(AlgHandle, *mut u8, u32, u32) -> NtStatus;

struct HostBcrypt {
    open: Option<OpenAlgFn>,
    close: Option<CloseAlgFn>,
    gen_random: Option<GenRandomFn>,
}

struct HandleEntry {
    host_handle: usize,
    is_rng: bool,
}

struct State {
    handles: HashMap<usize, HandleEntry>,
    next_id: usize,
    seeded: Option<u64>,
}

static HOST: LazyLock<HostBcrypt> = LazyLock::new(|| {
    // SAFETY: loading the system `bcrypt.dll` from System32 only.
    unsafe {
        let name: Vec<u16> = "bcrypt.dll\0".encode_utf16().collect();
        let m = LoadLibraryExW(name.as_ptr(), std::ptr::null_mut(), LOAD_LIBRARY_SEARCH_SYSTEM32);
        if m.is_null() {
            return HostBcrypt {
                open: None,
                close: None,
                gen_random: None,
            };
        }
        let open = GetProcAddress(m, c"BCryptOpenAlgorithmProvider".as_ptr().cast())
            .map(|p| std::mem::transmute::<_, OpenAlgFn>(p));
        let close = GetProcAddress(m, c"BCryptCloseAlgorithmProvider".as_ptr().cast())
            .map(|p| std::mem::transmute::<_, CloseAlgFn>(p));
        let gen_random = GetProcAddress(m, c"BCryptGenRandom".as_ptr().cast())
            .map(|p| std::mem::transmute::<_, GenRandomFn>(p));
        HostBcrypt {
            open,
            close,
            gen_random,
        }
    }
});

fn parse_seed(text: &str) -> Option<u64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(hex) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        trimmed.parse::<u64>().ok()
    }
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    let seeded = std::env::var("VICTORY_RNG_SEED")
        .ok()
        .and_then(|v| parse_seed(&v))
        .map(|s| if s == 0 { 0x9E37_79B9_7F4A_7C15 } else { s });
    if let Some(seed) = seeded {
        tracing::info!(seed = format_args!("0x{seed:x}"), "bcrypt: deterministic VICTORY_RNG_SEED enabled");
    }
    Mutex::new(State {
        handles: HashMap::new(),
        next_id: 0xBC00_0001,
        seeded,
    })
});

fn next_splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn fill_seeded(state: &mut u64, buf: &mut [u8]) {
    let mut offset = 0;
    while offset < buf.len() {
        let word = next_splitmix64(state).to_le_bytes();
        let take = (buf.len() - offset).min(word.len());
        buf[offset..offset + take].copy_from_slice(&word[..take]);
        offset += take;
    }
}

fn is_rng_alg(alg_id: *const u16) -> bool {
    if alg_id.is_null() {
        return false;
    }
    let mut len = 0usize;
    // SAFETY: caller passes a NUL-terminated UTF-16 algorithm name.
    while len < 64 && unsafe { *alg_id.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: slice covers up to the NUL terminator.
    let slice = unsafe { std::slice::from_raw_parts(alg_id, len) };
    String::from_utf16_lossy(slice).eq_ignore_ascii_case("RNG")
}

pub extern "system" fn BCryptOpenAlgorithmProvider(
    ph_algorithm: *mut AlgHandle,
    psz_alg_id: *const u16,
    psz_implementation: *const u16,
    dw_flags: u32,
) -> NtStatus {
    if ph_algorithm.is_null() || psz_alg_id.is_null() {
        return STATUS_INVALID_PARAMETER;
    }
    // SAFETY: checked non-null above.
    unsafe { *ph_algorithm = std::ptr::null_mut() };

    let is_rng = is_rng_alg(psz_alg_id);
    let seeded_mode = STATE.lock().unwrap().seeded.is_some();
    let mut host_handle = std::ptr::null_mut();

    if !(seeded_mode && is_rng) {
        let Some(open) = HOST.open else {
            return if is_rng { STATUS_UNSUCCESSFUL } else { STATUS_NOT_FOUND };
        };
        // SAFETY: forwarding validated pointers to system `BCryptOpenAlgorithmProvider`.
        let st = unsafe { open(&mut host_handle, psz_alg_id, psz_implementation, dw_flags) };
        if st < 0 {
            return st;
        }
    }

    let mut state = STATE.lock().unwrap();
    let id = state.next_id;
    state.next_id += 1;
    state.handles.insert(
        id,
        HandleEntry {
            host_handle: host_handle as usize,
            is_rng,
        },
    );
    // SAFETY: checked non-null above.
    unsafe { *ph_algorithm = id as AlgHandle };
    STATUS_SUCCESS
}

pub extern "system" fn BCryptCloseAlgorithmProvider(
    h_algorithm: AlgHandle,
    dw_flags: u32,
) -> NtStatus {
    if h_algorithm.is_null() {
        return STATUS_INVALID_HANDLE;
    }
    let entry = STATE.lock().unwrap().handles.remove(&(h_algorithm as usize));
    let Some(entry) = entry else {
        return STATUS_INVALID_HANDLE;
    };
    if entry.host_handle != 0
        && let Some(close) = HOST.close
    {
        // SAFETY: `entry.host_handle` was returned by `BCryptOpenAlgorithmProvider`.
        return unsafe { close(entry.host_handle as AlgHandle, dw_flags) };
    }
    STATUS_SUCCESS
}

pub extern "system" fn BCryptGenRandom(
    h_algorithm: AlgHandle,
    pb_buffer: *mut u8,
    cb_buffer: u32,
    dw_flags: u32,
) -> NtStatus {
    if pb_buffer.is_null() {
        return STATUS_INVALID_PARAMETER;
    }
    let use_system = (dw_flags & BCRYPT_USE_SYSTEM_PREFERRED_RNG) != 0;
    let host_handle = if use_system {
        if !h_algorithm.is_null() {
            return STATUS_INVALID_PARAMETER;
        }
        std::ptr::null_mut()
    } else {
        if h_algorithm.is_null() {
            return STATUS_INVALID_HANDLE;
        }
        let state = STATE.lock().unwrap();
        let Some(entry) = state.handles.get(&(h_algorithm as usize)) else {
            return STATUS_INVALID_HANDLE;
        };
        if !entry.is_rng {
            return STATUS_INVALID_HANDLE;
        }
        entry.host_handle as AlgHandle
    };

    if cb_buffer == 0 {
        return STATUS_SUCCESS;
    }

    {
        let mut state = STATE.lock().unwrap();
        if let Some(seed) = state.seeded.as_mut() {
            // SAFETY: `pb_buffer` is non-null and `cb_buffer` bytes were requested by the caller.
            let buf = unsafe { std::slice::from_raw_parts_mut(pb_buffer, cb_buffer as usize) };
            fill_seeded(seed, buf);
            return STATUS_SUCCESS;
        }
    }

    if let Some(gen_random) = HOST.gen_random {
        // SAFETY: forwarding validated buffer and handle to system `BCryptGenRandom`.
        return unsafe { gen_random(host_handle, pb_buffer, cb_buffer, dw_flags) };
    }
    STATUS_UNSUCCESSFUL
}

pub(crate) fn module() -> HleModule {
    HleModule {
        name: "bcrypt",
        dlls: &["bcrypt.dll"],
        exports: vec![
            hle_export!(BCryptOpenAlgorithmProvider),
            hle_export!(BCryptCloseAlgorithmProvider),
            hle_export!(BCryptGenRandom),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_invalid_and_double_closed_handles() {
        assert_eq!(
            BCryptCloseAlgorithmProvider(std::ptr::null_mut(), 0),
            STATUS_INVALID_HANDLE
        );
        assert_eq!(
            BCryptCloseAlgorithmProvider(0xDEAD_BEEFusize as AlgHandle, 0),
            STATUS_INVALID_HANDLE
        );

        let rng: Vec<u16> = "RNG\0".encode_utf16().collect();
        let mut alg = std::ptr::null_mut();
        assert_eq!(
            BCryptOpenAlgorithmProvider(&mut alg, rng.as_ptr(), std::ptr::null(), 0),
            STATUS_SUCCESS
        );
        assert!(!alg.is_null());

        let mut buf = [0u8; 16];
        assert_eq!(BCryptGenRandom(alg, buf.as_mut_ptr(), 16, 0), STATUS_SUCCESS);
        assert_eq!(BCryptCloseAlgorithmProvider(alg, 0), STATUS_SUCCESS);
        // Double close and post-close use must return STATUS_INVALID_HANDLE.
        assert_eq!(BCryptCloseAlgorithmProvider(alg, 0), STATUS_INVALID_HANDLE);
        assert_eq!(
            BCryptGenRandom(alg, buf.as_mut_ptr(), 16, 0),
            STATUS_INVALID_HANDLE
        );
    }

    #[test]
    fn system_preferred_rng_requires_null_handle() {
        let mut buf = [0u8; 8];
        assert_eq!(
            BCryptGenRandom(
                0x1234usize as AlgHandle,
                buf.as_mut_ptr(),
                8,
                BCRYPT_USE_SYSTEM_PREFERRED_RNG
            ),
            STATUS_INVALID_PARAMETER
        );
        assert_eq!(
            BCryptGenRandom(
                std::ptr::null_mut(),
                buf.as_mut_ptr(),
                8,
                BCRYPT_USE_SYSTEM_PREFERRED_RNG
            ),
            STATUS_SUCCESS
        );
    }

    #[test]
    fn seeded_prng_is_deterministic() {
        let mut s1 = 42u64;
        let mut s2 = 42u64;
        let mut a = [0u8; 24];
        let mut b = [0u8; 24];
        fill_seeded(&mut s1, &mut a);
        fill_seeded(&mut s2, &mut b);
        assert_eq!(a, b);
        assert_eq!(parse_seed("0x2a"), Some(42));
        assert_eq!(parse_seed("42"), Some(42));
        assert_eq!(parse_seed(""), None);
    }
}
