# Linux status of this fork

What builds and passes on an x86-64 Linux host, and what runs under Proton
(UMU). Re-run after each sync; each section says when it was checked.

## This branch (`kc/nvidia`)

Based on hyphennn's NVIDIA fork ([fxceless/victory-fork](https://github.com/fxceless/victory-fork)
`main` at `81d54b06`), the only public tree measured on a GeForce card. Its
history is kept with authors and dates, rewritten in two ways only:
`diagnostics/` (run logs of the title's output) is dropped from every
commit, and the one commit message naming the title says "one title"
instead. Each rewritten commit names its original. Commits that only
touched `diagnostics/` fall away (138 → 120). Outside `diagnostics/` the
tree is byte-identical to hyphennn's. The README's links into
`diagnostics/` resolve only in hyphennn's repository.

hyphennn's base is upstream `ffe310eb` (2026-11-24); upstream has moved on
since. On top come this fork's Linux commits (`kc/linux`) and upstream
`3f6910ac` (victory exits with the title's status, which `proton-smoke.sh`
expects).

## Host setup used

- Rust: the pinned toolchain from `rust-toolchain.toml`, through a separate
  rustup (see `scripts/kc-env.sh`), plus the `x86_64-pc-windows-gnullvm` and
  `x86_64-pc-windows-msvc` targets.
- Linker: LLVM-MinGW (UCRT) for gnullvm.
- Runtime: `umu-run` (umu-launcher) with its default UMU-Proton.
- `. scripts/kc-env.sh` before every command below.

## Cross builds (checked 2026-11-26)

| Build | Result |
|---|---|
| `cargo build --release -p victory --target x86_64-pc-windows-gnullvm` | ✅ links: PE32+ x86-64 console |
| `cargo xwin build --release -p victory --target x86_64-pc-windows-msvc` | not run (needs Microsoft's licence accepted) |

## Native Linux (checked 2026-11-26)

`victory-gpu` and `victory-loader` do not build natively (`victory-loader`:
`crash.rs` uses `windows_sys` ungated, and `lib.rs` names `console_insn`,
which is Windows-only). Native tests for the crates that build:

```bash
L="-p victory-core -p victory-pe -p victory-formats -p victory-pipeline -p victory-game -p xtask"
cargo test --no-fail-fast --locked $L -- --exact \
  --skip image::tests::parses_own_test_binary --skip image::tests::rva_to_file_offset_maps_entry_point \
  --skip image::tests::rva_to_file_offset_rejects_offsets_past_4_gib --skip image::tests::rejects_iat_outside_image
```

Result: **53 passed, 0 failed, 4 ignored** (checked 2026-11-26, after the
policy's per-file limit went to 1 MiB: two of hyphennn's source files are
past 512 KiB). The 4 skipped tests parse the test executable as a PE,
which is ELF on Linux.

## Under Proton (checked 2026-11-26)

| Check | Result |
|---|---|
| `bash tests/proton-launcher.sh` (fake umu-run, no Wine) | ✅ passed |
| `tests/proton-smoke.sh` (synthetic title, expects `title exited with 42`) | ✅ passed on UMU-Proton 11.0-2e: imports bound `hle=1 host=1 missing=0`, exit status 42 |
| `load_probe` (gnullvm test exe + prebuilt `hle_probe.dll`, run with `umu-run`, `VICTORY_HLE_PROBE`) | ✅ 2 passed, 0 failed |
| `hlsl_probe` (victory-gpu lib test) | ✅ builtin `d3dcompiler_47` refuses 5 of 5; the runtime's path (`hlsl::compile_compute`) compiles 5 of 5 with DXC |
| helper tests on the card (`VICTORY_TEST_ADAPTER=hardware`, RTX 3080 Ti, vkd3d-proton) | ✅ `indirect::tests::`, 20 passed: clamp, masked patch, card compare ×2, card fetch, gathered records, budget, lane order, windows, spread |

`proton-smoke.sh` runs the `release/victory.exe` already built; rebuild
it first. Proton also prints two harmless lines on a fresh prefix: it
can't copy `amdxcffx64.dll` (AMD-only), and "unable to use parent for game
drive".

How `load_probe` was run:

```bash
cargo test --locked --no-run -p victory-loader --test load_probe --target x86_64-pc-windows-gnullvm
cargo build --manifest-path tests/fixtures/hle-probe/Cargo.toml \
  --target x86_64-pc-windows-gnullvm --target-dir "$CARGO_TARGET_DIR/fixtures/hle-probe"
P="$CARGO_TARGET_DIR/fixtures/hle-probe/x86_64-pc-windows-gnullvm/debug/hle_probe.dll"
T=$(ls -t "$CARGO_TARGET_DIR"/x86_64-pc-windows-gnullvm/debug/deps/load_probe-*.exe | head -1)
VICTORY_HLE_PROBE="Z:${P//\//\\}" umu-run "Z:${T//\//\\}"
```

## Notes

- **Runtime HLSL helpers under Proton: fixed with DXC (2026-11-26).**
  This tree compiles five helpers at runtime, all `cs_5_1` through
  `D3DCompile`. Wine's builtin `d3dcompiler_47` refuses all five, each with
  E5005 (a method not defined on `RWByteAddressBuffer`):

  | Helper | Refused at | Effect at runtime without DXC |
  |---|---|---|
  | indirect argument clamp | `clamp:198:10 'InterlockedOr'` | every indirect draw and dispatch dropped |
  | masked patch (`VICTORY_MASKED_PATCH`, default on) | `masked patch:19:33 'Load'` | the patch is not sent |
  | card compare | `card compare:33:62 'Load4'` | no host copies kept in the card's memory |
  | card compare, wide (`VICTORY_CARD_HASH_WIDE`) | `card compare wide:42:59 'Load4'` | as above |
  | card fetch in flight (`VICTORY_CARD_FETCH_INFLIGHT=1`) | `card fetch in flight:27:16 'Load'` | the old fetch is used |

  They now go through `crate::hlsl::compile_compute` (after jman
  `dffc5924`): the system's compiler first, as before, so Windows is
  unchanged; then DXC (`cs_6_0`) from `VICTORY_DXC`. Under UMU-Proton 11.0-2e,
  DXC compiled all five (clamp 7400, masked patch 3680, card compare 4296,
  wide 5000, card fetch 4904 bytes), and the tests that run them on the
  card pass (table above). `VICTORY_DXC` names `dxcompiler.dll` or its
  folder (with `dxil.dll` beside it), from Microsoft's open-source
  [DirectXShaderCompiler releases](https://github.com/microsoft/DirectXShaderCompiler/releases)
  (`bin/x64`). `scripts/victory-proton.sh` turns a Linux path into the
  `Z:` path Wine needs. Not yet tested: a real title run with it.

  The depth plane's three shaders compile with the builtin (checked with a
  local build; `host::compile` makes them with the system's compiler).

  `winetricks d3dcompiler_47` is **not** a fix: it installs the Windows 8.1
  SDK compiler (6.3.9600.16384), which predates Shader Model 5.1 and
  refuses every `_5_1` target (`X3506`), so it also breaks what the
  builtin compiles. A prefix that has had that verb run needs
  `d3dcompiler_47` set back to `builtin`.

  How it was run (after `. scripts/kc-env.sh`, which sets `VICTORY_DXC`):

  ```bash
  cargo test --locked --no-run -p victory-gpu --lib --target x86_64-pc-windows-gnullvm
  T=$(ls -t "$CARGO_TARGET_DIR"/x86_64-pc-windows-gnullvm/debug/deps/victory_gpu-*.exe | head -1)
  umu-run "Z:${T//\//\\}" hlsl --nocapture --test-threads=1
  VICTORY_TEST_ADAPTER=hardware umu-run "Z:${T//\//\\}" indirect::tests:: --test-threads=1
  ```
