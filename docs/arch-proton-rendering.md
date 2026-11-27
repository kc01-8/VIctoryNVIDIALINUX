# Cross-building on Arch Linux and checking rendering under Proton

On CachyOS x86-64, VIctory at `d2a4bae` built for Windows MSVC and rendered
through loading on a GeForce RTX 4080 (NVIDIA 620.70.06). Entering the world
triggered a GPU fault. This is one compatibility report. See the
[world-rendering status and GPU hang warning](../README.md#status-2026-11-23)
before running a title.

## Build the launcher

The host had AVX2, Clang/LLVM/LLD, CMake, Ninja, Wine and `file`. Check
`df -h .` and the filesystem holding your SDK cache. This release build used
365 MiB for `target/` and 642 MiB for the x64 SDK/CRT cache, measured
separately. Allow extra space for Rust, Cargo downloads and temporary files.

Use the compiler and target standard library from the **same Rust distribution**.
Combining Arch's Rust 1.98.1 with the official 1.98.1 Windows standard library
failed with `E0514` on `compiler_builtins`, even with matching version numbers.
The official 1.98.1 compiler, host standard library and Windows standard
library worked together. With rustup, run these commands from the checkout
in Bash. Check the [Microsoft license notice](https://github.com/rust-cross/cargo-xwin#cargo-xwin)
before installing or using cargo-xwin.

```bash
repo="$PWD"
target=x86_64-pc-windows-msvc
rustup toolchain install 1.98.1 --profile minimal --target "$target"
cargo +1.98.1 fetch --locked --target "$target"
cargo +1.98.1 install --locked --version 0.24.1 cargo-xwin
export XWIN_ARCH=x86_64
export XWIN_CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/victory-xwin-x64"
cargo +1.98.1 xwin build --release --locked --target "$target" -p victory
exe="$repo/target/$target/release/victory.exe"
file "$exe"
du -sh "$repo/target" "$XWIN_CACHE_DIR"
```

`file` should report a PE32+ x86-64 console executable. If Cargo fails to
resolve crates.io, run `curl -fsS https://index.crates.io/config.json` in the
build shell and your normal terminal. In this test the terminal had network
access and could fetch the locked dependencies.

## Check the CLI and game path

System Wine 11.21 ran `--help` and `config` successfully in a dedicated
prefix. It later opened the title's archives and displayed a white window.
`D3D12CreateDevice` at feature level 12.0 returned `0x80070057`, and VIctory
logged `drawn=0` as the frame counter advanced.

The working graphics configuration used Steam's Proton Experimental with
vkd3d-proton and DXVK. First check that `vulkaninfo --summary` lists your
GPU. The example continues in the same Bash shell and uses a separate prefix.
Adjust the Steam library path if needed. This Proton build used
`SteamLinuxRuntime_4`; its `toolmanifest.vdf` identifies the runtime app ID.

```bash
game='/absolute/path/to/your/game'
work="${XDG_DATA_HOME:-$HOME/.local/share}/victory-proton-test"
steam="$HOME/.local/share/Steam"
common="$steam/steamapps/common"
runtime="$common/SteamLinuxRuntime_4/_v2-entry-point"
proton="$common/Proton - Experimental/proton"
unset WINEPREFIX VICTORY_GAME_DIR VICTORY_CACHE_DIR VICTORY_DATA_DIR VICTORY_TEST_TITLES
export STEAM_COMPAT_CLIENT_INSTALL_PATH="$steam"
export STEAM_COMPAT_DATA_PATH="$work/compatdata"
export STEAM_COMPAT_INSTALL_PATH="$repo"
export STEAM_COMPAT_MOUNTS="$repo:$game:$work${STEAM_COMPAT_MOUNTS:+:$STEAM_COMPAT_MOUNTS}"
mkdir -p "$STEAM_COMPAT_DATA_PATH"
cd "$work"
slr() { "$runtime" --verb=run -- "$proton" "$@"; }
slr run cmd.exe /d /c ver
export WINEDLLOVERRIDES='d3d12,d3d12core,dxgi=n'
slr runinprefix "$exe" --help
slr getcompatpath "$game"
```

For a fresh prefix, copy `"$repo/victory.example.toml"` to
`"$STEAM_COMPAT_DATA_PATH/pfx/drive_c/victory.toml"`. Set `[paths].game_dir`
to the Windows path printed by `getcompatpath` (Proton's `winepath -w`).
Use a single-quoted TOML string to preserve backslashes. Keep the game files
outside the checkout and run the following check before launching.

```bash
export VICTORY_CONFIG='C:\victory.toml'
slr runinprefix "$exe" config
```

Check the printed config filename and game directory. If Proton returns zero
with no console output, redirect inside the prefix and read the file on the
host. This example only reads configuration.

```bash
exe_win=$(slr getcompatpath "$exe" | tr -d '\r')
printf '@echo off\r\n"%s" config > C:\\victory-config.txt 2>&1\r\n' "$exe_win" \
    > "$STEAM_COMPAT_DATA_PATH/pfx/drive_c/victory-config.cmd"
slr runinprefix cmd.exe /d /c 'C:\victory-config.cmd'
cat "$STEAM_COMPAT_DATA_PATH/pfx/drive_c/victory-config.txt"
```

## Distinguish a white window from a world-loading crash

With `runinprefix`, specifying the native DLL overrides above made the
difference in the device probe. The loader trace showed native `d3d12.dll`,
`d3d12core.dll` and `dxgi.dll`. A standalone probe then created the feature
level 12.0 device, direct queue, fence, event and both descriptor heaps.
`--help` and `config` alone do not exercise these calls.

For an intentional title run, read the executable name from the game's
manifest (such as `MicrosoftGame.config`). Keep the same shell, prefix and
overrides. Substitute the manifest's relative executable path below.

```bash
title_win=$(slr getcompatpath "$game/relative/path/from/manifest.exe" | tr -d '\r')
slr runinprefix "$exe" run "$title_win" --execute
```

In the Proton run, `drawn` and `dispatched` rose with the submitted work.
At frame 4096 all 56,069 draws and 2,884 dispatches in the reporting interval
had executed. During the transition to the world, two compute pipelines
were refused. The Linux kernel then logged NVIDIA Xid 13 (GPU MMU fault) for
the title process, and VIctory recorded a null write (`0xc0000005`) in
`crashes.log`. The logs do not identify the GPU command that faulted.

To compare a run, match its timestamps against
`journalctl -k --utc -g 'NVRM: Xid'` and the title's `crashes.log` under
`%LOCALAPPDATA%\victory\data` in the same prefix. For this build, successful
menu and loading-screen rendering is the last verified milestone.
