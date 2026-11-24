# Known Issues & Integration Roadmap

Tracked issues for the merged **VIctory + XCL** workspace (`shadshuvo/GanduTheftAutoVi`).

---

## Open Issues

### #2 — Direct DXIL Backend (`crates/xcl/src/gpu/dxil/`) Completion (Phases 4–7)
- **Problem:** `gpu/dxil/` Phases 0–3 (LLVM 3.7 bitstream/IR, compute ALU/memory/cross-lane ops) are done, while Phases 4–6 (`XCL_DIRECT_DXIL=1` live routing for real compute, VS/PS signatures, mesh two-pass pipelines) are partial and Phase 7 (default-on parity with HLSL/DXC) is pending.
- **Action:** Complete Phases 4–7 and measure corpus coverage via `xcl shaders <pack> --direct`.

### #6 — Consolidate `XGameRuntime` GDK Thunk Table & Audio Decoders
- **Problem:** `crates/xcl/src/hle/xgameruntime/` (396-function GDK table, `XGameSave`) and `crates/victory-hle/src/modules/xgameruntime.rs` duplicate runtime services, as do the `xma2` / `opus` decoders.
- **Action:** Unify onto a single shared GDK runtime and audio decoder implementation.

### #9 — Critical: Root Signature ABI Mismatch for XCL-Fallback Shaders in `victory-gpu`
- **Problem:** `translate_via_xcl()` in `crates/victory-gpu/src/shader_cache.rs` outputs DXIL compiled for **XCL's ABI** (`u0 io`, `u1 res`, `u2 mem` + directly indexed heaps), whereas `HostPipeline::new()` in `crates/victory-gpu/src/pipeline/host.rs` builds a root signature for **VIctory's ABI** (`b0` root constants + `ROOT_CBV_SPACE` descriptors).
- **Impact:** PSO creation or draw-time binding for any shader that falls back to XCL will fail or read uninitialized `io`/`mem` UAVs.
- **Action:** Track whether a `HostShader` uses XCL's ABI and have `HostPipeline::new()` / `draw.rs` bind `xcl::gpu::recomp::pso::abi_root_signature()` + `io`/`mem` buffers for those pipelines.

### #10 — `apps/victory/Cargo.toml` Unconditional `xcl` Dependency
- **Problem:** `victory-gpu` gates `xcl` behind the `xcl-shaders` feature, but `apps/victory/Cargo.toml` still links `xcl` unconditionally, pulling `GPL-2.0-only` code into `victory.exe` (`GPL-3.0-or-later`).
- **Action:** Gate `xcl` in `apps/victory/Cargo.toml` behind an optional feature (e.g. `xcl-bridge`) so pure `GPL-3.0-or-later` builds of `victory` remain clean.

### #11 — Missing UMA Readback Sanity Check (`check_uma_import`) in `victory-gpu`
- **Problem:** `Host::create()` in `crates/victory-gpu/src/host.rs` opens UMA windows via `OpenExistingHeapFromAddress` without verifying GPU visibility (unlike XCL's `check_uma_import`). Under Wine/Proton without `userfaultfd`, `OpenExistingHeapFromAddress` can succeed while the GPU reads zeros.
- **Action:** Add a lightweight CPU-write → GPU-readback check on UMA initialization and fall back to shadow buffers if the check fails.

### #12 — Depth Bounds Test Stripped in `GraphicsState`
- **Problem:** Subobject `21` (`D3D12_DEPTH_STENCIL_DESC1`) in `crates/victory-gpu/src/pipeline/host.rs` drops `DepthBoundsTestEnable` when converted to `D3D12_DEPTH_STENCIL_DESC`.
- **Action:** Preserve `DepthBoundsTestEnable` using a pipeline-state stream (`CreatePipelineState`) when enabled.

---

## Resolved Issues

- **#3** — Wired `victory-game::sgd5` into `xcl shaders` and XCL Direct DXIL / recomp fallback into `victory-gpu/src/shader_cache.rs` (`f51aaad`).
- **#4** — Shared RPF9 scanner (`rpf9.rs`) and GFX10 tiling + tables (`tiling.rs`, `tiling_tables.rs`) in `victory-formats` across `victory` and `xcl` (`61d1feb`).
- **#5** — Ported lazy physical-page commit (`lazy.rs`) into `victory-hle` and `rdpru`/`rdpid`/`clzero`-loop Zen 2 emulation into `victory-loader` (`44fd82f`).
- **#7** — Wired `VirtualFree` UMA hook, `XMemVirtualAlloc` UMA routing, `Wave64`-on-`Wave32` device detection, and immediate arena sweep (`6d45a03`).
- **#8** — Set explicit `GPL-2.0-only` on `crates/xcl` / `tests/xcl/*` and gated `xcl` behind `xcl-shaders` in `victory-gpu` (`c97ac76`).
