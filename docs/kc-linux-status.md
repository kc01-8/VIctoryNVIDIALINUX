# Linux status of this fork

What builds and passes on an x86-64 Linux host, and what runs under Proton
(UMU). Re-run after upstream merges; each section says when it was checked.

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
| `cargo build --release -p victory --target x86_64-pc-windows-gnullvm` | ✅ links: PE32+ x86-64 console. One harmless linker note: clang ignores rustc's `-no-pie` |
| `cargo xwin build --release -p victory --target x86_64-pc-windows-msvc` | not run yet |

## Native Linux (checked 2026-11-26)

`cargo check --workspace --locked` fails in `victory-gpu` (4 errors: the
Windows-only `host` module, `host_resource`, `IDLE_FRAMES`, `IDLE_MS`).
`victory-hle` and `apps/victory` depend on it. Upstream PR #235's
`cfg(windows)` gates address this; it has not been ported.

Native tests for the crates that build:

```bash
L="-p victory-core -p victory-pe -p victory-formats -p victory-pipeline -p victory-game -p victory-loader -p xtask"
cargo test --locked $L -- --exact \
  --skip image::tests::parses_own_test_binary --skip image::tests::rva_to_file_offset_maps_entry_point \
  --skip image::tests::rva_to_file_offset_rejects_offsets_past_4_gib --skip image::tests::rejects_iat_outside_image \
  --skip prepare::tests::patches_own_test_binary --skip prepare::tests::keeps_iat_slots_when_import_size_excludes_the_terminator \
  --skip prepare::tests::file_name_is_stable_per_build
```

Result: **121 passed, 0 failed, 5 ignored**. The 7 skipped tests parse the
test executable as a PE, which is ELF on Linux.

`victory-shader` is no longer in that list. Its lib tests now use the DXIL
validator (`emit::validator`), which is Windows-only, so they do not compile
natively (6 × E0433). `cargo clippy --lib -p victory-shader` is clean.

## Under Proton (checked 2026-11-26)

| Check | Result |
|---|---|
| `bash tests/proton-launcher.sh` (fake umu-run, no Wine) | ✅ passed |
| `tests/proton-smoke.sh` (synthetic title, expects `title exited with 42`) | ✅ passed on UMU-Proton 11.0-2e: imports bound `hle=1 host=1 missing=0`, exit status 42 |
| `load_probe` (gnullvm test exe + prebuilt `hle_probe.dll`, run with `umu-run`, `VICTORY_HLE_PROBE`) | ✅ 2 passed, 0 failed |

`proton-smoke.sh` needed a fix here: since upstream `3f6910ac`, victory exits
with the title's own status, so the launcher's correct 42 failed the script
under `pipefail`. Proton also prints two harmless lines on a fresh prefix:
it can't copy `amdxcffx64.dll` (AMD-only), and "unable to use parent for game
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

- **Runtime HLSL helpers under Proton (blocker for real runs).** Measured
  with `hlsl_probe` (victory-gpu lib test) on UMU-Proton 11.0-2e and the
  builtin `d3dcompiler_47`:

  | Helper | Result | Effect at runtime |
  |---|---|---|
  | indirect argument clamp (`indirect.rs`) | ❌ `clamp:58:32 E5005 'Load'` on `RWByteAddressBuffer` | every indirect draw and dispatch dropped |
  | depth twin converter (`twin.rs`) | ❌ `depth:17:19 E5005 'Load'` on `RWByteAddressBuffer` | no depth twins |
  | present compositor vs/ps (`host.rs`) | ✅ | none |

  The other seven `D3DCompile` calls in victory-gpu are test-only. Options:
  Microsoft's `d3dcompiler_47.dll` with `WINEDLLOVERRIDES=d3dcompiler_47=n,b`,
  or one shared helper-compile path with DXC (after jman `dffc5924`, extended
  to the twin converter). Run the probe again with whichever fix is chosen:
  `umu-run Z:\...\victory_gpu-*.exe hlsl_probe --nocapture`.
- This tree has no `VICTORY_DXC`. The indirect-argument clamp compiles with
  `D3DCompile` (`crates/victory-gpu/src/indirect.rs`), and only the validator
  path reads `VICTORY_DXIL`. jman's fork added `VICTORY_DXC` and embedded
  helper shaders (`dffc5924`); not ported.
