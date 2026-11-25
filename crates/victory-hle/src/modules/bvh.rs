//! `XboxBvhBuilder.dll`: XDXR ray-tracing acceleration structure builder shims.
//!
//! Serves the 7 mangled `XboxBvhBuilder` exports imported by `game_scarlett_master.exe`
//! without loading the bundled DLL (whose 52 console compute programs take 25–60s
//! to compile on the title's thread). Set `VICTORY_BVH_BUILDER=console` to opt out.
//!
//! Adapted from Leonida (`runtime/shims/xbox_bvh_builder.cpp`, `docs/xdxr.md`,
//! GPL-3.0-or-later).
#![allow(non_snake_case)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{LazyLock, Mutex};

use crate::{HleExport, HleModule};

const E_NOTIMPL: i32 = 0x8000_4001_u32 as i32;
pub const POSTBUILD_INFO_OFFLINE_STATISTICS: u32 = 0x10;

/// `D3D12XBOX_XDXR_STREAM_DESC` (`0x10` bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StreamDesc {
    pub size_in_bytes: u64,
    pub stream: *const c_void,
}

/// `D3D12_RAYTRACING_ACCELERATION_STRUCTURE_POSTBUILD_INFO_DESC` (`0x10` bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PostbuildInfoDesc {
    pub dest_buffer: u64,
    pub info_type: u32,
    pub _pad: u32,
}

/// `D3D12XBOX_BUILD_RAYTRACING_ACCELERATION_STRUCTURE_BATCHED_DESC2` (`0x50` bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BatchedDesc2 {
    pub dest_as_data: u64,
    pub inputs: [u64; 3],
    pub source_as_data: u64,
    pub scratch_as_data: u64,
    pub num_postbuild_info_descs: u32,
    pub _pad: u32,
    pub postbuild_info_descs: *const PostbuildInfoDesc,
    pub stream: StreamDesc,
}

#[derive(Debug, Clone, Copy)]
struct BuilderState {
    device: usize,
    morton64: bool,
}

static BUILDERS: LazyLock<Mutex<HashMap<usize, BuilderState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn enabled() -> bool {
    !std::env::var("VICTORY_BVH_BUILDER")
        .is_ok_and(|v| v.trim().eq_ignore_ascii_case("console"))
}

/// `XboxBvhBuilder::XboxBvhBuilder(ID3D12Device11*, bool)`.
pub extern "system" fn construct(this: *mut c_void, device: *mut c_void, _flag: bool) -> *mut c_void {
    if !this.is_null() {
        BUILDERS.lock().unwrap().insert(
            this as usize,
            BuilderState {
                device: device as usize,
                morton64: false,
            },
        );
    }
    this
}

/// `XboxBvhBuilder::~XboxBvhBuilder()`.
pub extern "system" fn destruct(this: *mut c_void) {
    if !this.is_null() {
        BUILDERS.lock().unwrap().remove(&(this as usize));
    }
}

/// `XboxBvhBuilder::Set64BitMortonCodes(bool)`.
pub extern "system" fn set_64bit_morton_codes(this: *mut c_void, enable: bool) {
    if let Some(state) = BUILDERS.lock().unwrap().get_mut(&(this as usize)) {
        state.morton64 = enable;
    }
}

/// `XboxBvhBuilder::GetRaytracingAccelerationStructurePrebuildInfo(...)`.
pub extern "system" fn get_prebuild_info(
    this: *mut c_void,
    _inputs: *const c_void,
    stream: *const StreamDesc,
    info: *mut [u64; 3],
) {
    if !info.is_null() {
        // SAFETY: caller provides a writable 24-byte `PREBUILD_INFO` struct.
        unsafe { *info = [0; 3] };
    }
    if !stream.is_null() {
        // SAFETY: caller provides a readable `StreamDesc`.
        let s = unsafe { *stream };
        if s.size_in_bytes > 0 && !s.stream.is_null() {
            tracing::debug!(size = s.size_in_bytes, "XboxBvhBuilder: XDXR stream settings ignored");
        }
    }
    let has_device = BUILDERS
        .lock()
        .unwrap()
        .get(&(this as usize))
        .is_some_and(|s| s.device != 0);
    if !has_device {
        tracing::debug!("XboxBvhBuilder::GetRaytracingAccelerationStructurePrebuildInfo without device");
    }
}

/// `XboxBvhBuilder::BuildRaytracingAccelerationStructureBatched2X(...)`.
pub extern "system" fn build_batched_2x(
    _this: *mut c_void,
    _list: *mut c_void,
    count: u32,
    descs: *const BatchedDesc2,
) {
    if count == 0 || descs.is_null() {
        return;
    }
    for i in 0..count as usize {
        // SAFETY: `descs` points to `count` contiguous `BatchedDesc2` elements.
        let elem = unsafe { *descs.add(i) };
        if !elem.postbuild_info_descs.is_null() && elem.num_postbuild_info_descs > 0 {
            let mut has_offline_stats = false;
            for p in 0..elem.num_postbuild_info_descs as usize {
                // SAFETY: `postbuild_info_descs` has `num_postbuild_info_descs` entries.
                let pb = unsafe { *elem.postbuild_info_descs.add(p) };
                if pb.info_type == POSTBUILD_INFO_OFFLINE_STATISTICS {
                    has_offline_stats = true;
                    break;
                }
            }
            if has_offline_stats {
                tracing::debug!(
                    index = i,
                    "XboxBvhBuilder: skipping element requesting offline statistics (0x10)"
                );
                continue;
            }
        }
    }
}

/// `XboxBvhBuilder::CopyRaytracingAccelerationStructure(...)`.
pub extern "system" fn copy_as(
    _this: *mut c_void,
    _list: *mut c_void,
    _dest: u64,
    _source: u64,
    _mode: u32,
) {
}

/// `XboxBvhBuilder::RehydrateRaytracingAccelerationStructureBatched2X(...)`.
pub extern "system" fn rehydrate_batched_2x(
    _this: *mut c_void,
    _list: *mut c_void,
    _count: u32,
    _descs: *const c_void,
    _sizes: *const u64,
) -> i32 {
    tracing::debug!("XboxBvhBuilder::RehydrateRaytracingAccelerationStructureBatched2X -> E_NOTIMPL");
    E_NOTIMPL
}

fn export(name: &'static str, addr: *const c_void) -> HleExport {
    HleExport {
        name,
        address: addr as usize,
    }
}

pub(crate) fn module() -> HleModule {
    HleModule {
        name: "bvh",
        dlls: &["xboxbvhbuilder.dll"],
        exports: vec![
            export(
                "??0XboxBvhBuilder@@QEAA@PEAUID3D12Device11@@_N@Z",
                construct as *const c_void,
            ),
            export("??1XboxBvhBuilder@@QEAA@XZ", destruct as *const c_void),
            export(
                "?Set64BitMortonCodes@XboxBvhBuilder@@QEAAX_N@Z",
                set_64bit_morton_codes as *const c_void,
            ),
            export(
                "?GetRaytracingAccelerationStructurePrebuildInfo@XboxBvhBuilder@@QEAAXPEBUD3D12_BUILD_RAYTRACING_ACCELERATION_STRUCTURE_INPUTS@@PEBUD3D12XBOX_XDXR_STREAM_DESC@@PEAUD3D12_RAYTRACING_ACCELERATION_STRUCTURE_PREBUILD_INFO@@@Z",
                get_prebuild_info as *const c_void,
            ),
            export(
                "?BuildRaytracingAccelerationStructureBatched2X@XboxBvhBuilder@@QEAAXPEAUID3D12GraphicsCommandList6@@IPEBUD3D12XBOX_BUILD_RAYTRACING_ACCELERATION_STRUCTURE_BATCHED_DESC2@@@Z",
                build_batched_2x as *const c_void,
            ),
            export(
                "?CopyRaytracingAccelerationStructure@XboxBvhBuilder@@QEAAXPEAUID3D12GraphicsCommandList4@@_K1W4D3D12_RAYTRACING_ACCELERATION_STRUCTURE_COPY_MODE@@@Z",
                copy_as as *const c_void,
            ),
            export(
                "?RehydrateRaytracingAccelerationStructureBatched2X@XboxBvhBuilder@@QEAAJPEAUID3D12GraphicsCommandList8@@IPEBUD3D12_BUILD_RAYTRACING_ACCELERATION_STRUCTURE_DESC@@PEB_K@Z",
                rehydrate_batched_2x as *const c_void,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn xdxr_abi_struct_sizes_match_xboxbvhbuilder_dll() {
        assert_eq!(size_of::<StreamDesc>(), 0x10);
        assert_eq!(size_of::<PostbuildInfoDesc>(), 0x10);
        assert_eq!(size_of::<BatchedDesc2>(), 0x50);
    }

    #[test]
    fn builder_lifecycle_and_rehydrate_e_notimpl() {
        let mut storage = [0u8; 64];
        let this = storage.as_mut_ptr().cast::<c_void>();
        assert_eq!(construct(this, 0x1000usize as *mut c_void, false), this);
        set_64bit_morton_codes(this, true);
        assert!(BUILDERS.lock().unwrap().get(&(this as usize)).unwrap().morton64);

        let mut prebuild = [99u64; 3];
        get_prebuild_info(this, std::ptr::null(), std::ptr::null(), &mut prebuild);
        assert_eq!(prebuild, [0, 0, 0]);

        assert_eq!(
            rehydrate_batched_2x(
                this,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                std::ptr::null()
            ),
            E_NOTIMPL
        );
        destruct(this);
        assert!(!BUILDERS.lock().unwrap().contains_key(&(this as usize)));
    }
}
