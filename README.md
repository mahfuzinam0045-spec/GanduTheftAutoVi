# GanduTheftAutoVi — VIctory + XCL Merged Workspace

This repository is a merged Rust workspace combining two open-source Xbox Series X|S → Windows compatibility layer projects:

| Project | Source | License |
|---------|--------|---------|
| **VIctory** | [Marvisak/VIctory](https://github.com/Marvisak/VIctory) | GPL-3.0-or-later |
| **XCL** | [xcl-dev/xcl](https://github.com/xcl-dev/xcl) | GPL-2.0-only |

## Workspace Structure

```
crates/
  victory-core/       # VIctory: core types, config, logging
  victory-pe/         # VIctory: PE image parsing
  victory-loader/     # VIctory: EXE/DLL loader
  victory-hle/        # VIctory: high-level emulation (HLE) of system calls
  victory-gpu/        # VIctory: GPU/D3D12 layer
  victory-shader/     # VIctory: RDNA2 -> DXIL shader translation
  victory-formats/    # VIctory: game file format parsers
  victory-pipeline/   # VIctory: D3D12 pipeline state cache
  victory-game/       # VIctory: game-specific integration
  xcl/                # XCL: Xbox Series X|S compatibility layer (loader + HLE)
apps/
  victory/            # VIctory: CLI front-end (inspect / census / run / translate)
tests/
  xcl/
    testtitle/        # XCL: synthetic console title for integration tests
    testgamedll/      # XCL: fake middleware DLL for integration tests
  fixtures/           # VIctory: standalone fixtures (excluded from workspace)
docs/
  xcl/                # XCL design docs
```

## Prerequisites

- **Windows x64**
- **Rust** (toolchain pinned to `1.98.1`, target `x86_64-pc-windows-msvc`) - see `rust-toolchain.toml`
- **Visual Studio Build Tools** with MSVC and Windows SDK
- **DXC** (DirectX Shader Compiler) for shader translation

## Building

```powershell
# Build everything
cargo build --release

# Build only VIctory CLI
cargo build --release -p victory

# Build only XCL
cargo build --release -p xcl
```

## Running Tests

```powershell
# All tests (VIctory + XCL)
cargo test

# VIctory tests only
cargo test -p victory-loader -p victory-shader -p victory-pe

# XCL tests only
cargo test -p xcl
```

## Notes

- Neither project includes game assets -- you must supply your own files.
- See `victory.example.toml` for the VIctory runtime configuration.
- See `docs/xcl/` for XCL design documentation.
- Both projects are research/compatibility layers under active development.
