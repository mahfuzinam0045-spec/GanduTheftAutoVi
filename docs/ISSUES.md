# Known Issues & Integration Roadmap

Tracked issues for the merged **VIctory + XCL** workspace (`shadshuvo/GanduTheftAutoVi`).

---

## Open Issues

### #2 — Direct DXIL Backend (`crates/xcl/src/gpu/dxil/`) Completion (Phases 4–7)
- **Problem:** `gpu/dxil/` Phases 0–3 (LLVM 3.7 bitstream/IR, compute ALU/memory/cross-lane ops) are done, while Phases 4–6 (`XCL_DIRECT_DXIL=1` live routing for real compute, VS/PS signatures, mesh two-pass pipelines) are partial and Phase 7 (default-on parity with HLSL/DXC) is pending.
- **Action:** Complete Phases 4–7 and measure corpus coverage via `xcl shaders <pack> --direct`.

### #6 — Consolidate `XGameRuntime` GDK Thunk Table & Audio Decoders
- **Problem:** `crates/xcl/src/hle/xgameruntime/` and `crates/victory-hle/src/modules/xgameruntime.rs` duplicate runtime services, as do the `xma2` / `opus` decoders. (`gdk-table` was extracted in `32140c5`, but full consolidation is still in progress.)
- **Action:** Finish unifying onto a single shared GDK runtime and audio decoder implementation.

### #13 — Missing `tests/xcl/data/**/*.bin` Synthetic Test Fixtures
- **Problem:** `cargo test -p xcl --lib` fails on `hle::bcpack::tests::fixtures_decode_to_their_lanes` (`crates/xcl/src/hle/bcpack/tests.rs:37`) because the 14 synthetic `.bin` fixtures in `tests/xcl/data/bcpack/` (15 `.bin` files total in `upstream/xcl/tests/data/`) were excluded by `.gitignore`'s `*.bin` rule during the merge, even though `xtask/src/policy.rs` already exempts `tests/xcl/`.
- **Action:** Add `!/tests/xcl/data/**/*.bin` to `.gitignore` and restore the synthetic `.bin` test fixtures.

---

## Resolved Issues

- **#3** — Wired `victory-game::sgd5` into `xcl shaders` and XCL Direct DXIL / recomp fallback into `victory-gpu/src/shader_cache.rs` (`f51aaad`).
- **#4** — Shared RPF9 scanner (`rpf9.rs`) and GFX10 tiling + tables (`tiling.rs`, `tiling_tables.rs`) in `victory-formats` across `victory` and `xcl` (`61d1feb`).
- **#5** — Ported lazy physical-page commit (`lazy.rs`) into `victory-hle` and `rdpru`/`rdpid`/`clzero`-loop Zen 2 emulation into `victory-loader` (`44fd82f`).
- **#7** — Wired `VirtualFree` UMA hook, `XMemVirtualAlloc` UMA routing, `Wave64`-on-`Wave32` device detection, and immediate arena sweep (`6d45a03`).
- **#8** — Set explicit `GPL-2.0-only` on `crates/xcl` / `tests/xcl/*` and gated `xcl` behind `xcl-shaders` in `victory-gpu` (`c97ac76`).
- **#9** — Tagged XCL-ABI shaders (`VSH2`) and bound XCL's `u0..u5` ABI root signature + UMA/io UAVs in `pipeline/host.rs` and `draw.rs` (`9017879`).
- **#10** — Gated `xcl` behind `xcl-bridge` in `apps/victory/Cargo.toml` (`de24fc8`).
- **#11** — Added `verify_uma_import()` GPU readback check on UMA initialization in `victory-gpu/src/host.rs` (`88a172a`).
- **#12** — Preserved `DepthBoundsTestEnable` via `D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_DEPTH_STENCIL1` stream in `pipeline/host.rs` (`88a172a`).
