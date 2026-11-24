# Known Issues & Integration Roadmap

Tracked issues for the merged **VIctory + XCL** workspace (`shadshuvo/GanduTheftAutoVi`).

---

## 1. Licensing Conflict: Workspace `GPL-3.0-or-later` vs. `crates/xcl` (`GPL-2.0-only`)
- **Problem:** Root `Cargo.toml` sets `license = "GPL-3.0-or-later"` at the workspace level, while `crates/xcl` contains `GPL-2.0-only` code (including `AnyPS5`-derived logic and `crates/xcl/src/gpu/dxil/`).
- **Impact:** `GPL-2.0-only` and `GPL-3.0-or-later` are link-incompatible; they can coexist in one repository as separate binaries (`victory.exe` and `xcl.exe`), but cannot be linked into a single binary.
- **Action:** Set per-crate `license` fields explicitly (`crates/xcl/Cargo.toml` → `GPL-2.0-only`, `crates/victory-*` → `GPL-3.0-or-later`) or reconcile licensing before distributing unified binaries.

## 2. Direct DXIL Backend (`crates/xcl/src/gpu/dxil/`) Completion (Phases 4–7)
- **Problem:** `gpu/dxil/` Phases 0–3 (LLVM 3.7 bitstream/IR, compute ALU/memory/cross-lane ops) are done, while Phases 4–6 (`XCL_DIRECT_DXIL=1` live routing for real compute, VS/PS signatures, mesh two-pass pipelines) are partial and Phase 7 (default-on parity with HLSL/DXC) is pending.
- **Impact:** Most shaders still fall back to the slower HLSL → DXC path on cache misses.
- **Action:** Complete Phases 4–7 and measure corpus coverage via `xcl shaders <pack> --direct`.

## 3. Wire `victory-game::sgd5` & `victory-pipeline` to XCL's Shader Pipeline / Direct DXIL
- **Problem:** `victory-game::sgd5` (GTA VI's `SGD5` shader container decoder) and `victory-pipeline` are not yet connected to `xcl::shaders` or `xcl::gpu::dxil`.
- **Impact:** `victory` still uses `victory-shader`'s incomplete translator (missing 64-bit image atomics, merged geometry stages, and ray-tracing builder shaders), dropping draws in the 3D world.
- **Action:** Route `victory-pipeline` cache misses through XCL's `gpu/dxil` (with `gpu/recomp` HLSL fallback) and expose `sgd5` to `xcl shaders`.

## 4. Share RPF9 Reader (`xcl::rpf`) and GFX10 Tiling (`xcl::gpu::tiling`) Across Both CLIs
- **Problem:** `crates/xcl/src/rpf.rs` and `crates/xcl/src/gpu/tiling.rs` live solely inside `crates/xcl` and are not shared with `victory-formats` / `apps/victory`.
- **Action:** Extract or expose shared RPF9 archive parsing and GFX10 swizzle/detiling tables so both `victory` and `xcl` share one verified implementation.

## 5. Port Lazy Physical-Page Commit & Zen 2 CPU Emulation to `victory-hle` / `victory-loader`
- **Problem:**
  - `victory-hle` eagerly commits large virtual allocations (~28 GB commit), whereas `crates/xcl/src/hle/kernelx/lazy.rs` commits pages on-demand via a vectored exception handler (~15–18 GB commit).
  - `victory-loader` lacks XCL's `insn.rs` / `eh.rs` emulation for console Zen 2 instructions (`clzero`, `rdpru`, `mwaitx`, `clwb`) on pre-Ice Lake Intel CPUs (`0xC000001D`).
- **Action:** Port `lazy.rs` and `insn.rs` emulation into `victory-hle` and `victory-loader`.

## 6. Consolidate `XGameRuntime` GDK Thunk Table & Audio Decoders
- **Problem:** `crates/xcl/src/hle/xgameruntime/` (396-function GDK table, `XGameSave`) and `crates/victory-hle/src/modules/xgameruntime.rs` duplicate runtime services, as do the `xma2` / `opus` decoders.
- **Action:** Unify onto a single shared GDK runtime and audio decoder implementation.

## 7. Remaining GPU / UMA / VRAM Gaps in `victory-gpu`
- **Problem:** While `uma.rs` and `vram.rs` are now wired into `victory-gpu` (`host.rs`, `shadow.rs`, `resource.rs`) and `XMemAllocDefault(ALLOC_32BIT)`:
  - `XMemVirtualAlloc` in `victory-hle/src/modules/xmem.rs` still allocates outside UMA (because `VirtualFree` requires `VirtualAlloc` base pointers), so those buffers still use the `Shadow` fallback.
  - `D32S8` / `R32G8X24` and MSAA targets cannot use 64 KiB reserved `Pages` and fall back to `Arenas`.
  - Render-target metadata planes and Wave64-on-Wave32 support (for 32-lane NVIDIA GPUs) are still missing in `victory-gpu` / `victory-shader`.
- **Action:** Add `VirtualFree` tracking for UMA-backed `XMemVirtualAlloc` regions, split `D32S8` targets or tighten arena lifetimes, and adopt XCL's Wave64-on-Wave32 pass.
