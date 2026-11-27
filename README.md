# **NO COPYRIGHTED CONTENT PERMITTED. 100% CORRECTLY LICENSED CODE ONLY.**

## **ALL IN ACCORDANCE WITH THE LICENSES OF THE FORKED PROJECTS: GPL-3.0-OR-LATER ([VIctory](https://github.com/Marvisak/VIctory), [fxceless/victory-fork](https://github.com/fxceless/victory-fork), [JMAN730/victory](https://github.com/JMAN730/victory), [foxhound](https://github.com/readmet3xt-lab/foxhound), [trustno1-coder/VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA)).**

## **NO PULL REQUESTS WILL BE ACCEPTED WHICH DO NOT STRICTLY COMPLY WITH THESE LICENSES.**

---

# VIctoryNVIDIALINUX

A fork of [VIctory](https://github.com/Marvisak/VIctory), the compatibility
layer that runs an Xbox Series title on a PC from files you supply, aimed at
one setup: **an NVIDIA GeForce card on x86-64 Linux, under Proton**.

`victory.exe` stays a Windows program. On Linux it runs under Proton
(through [umu-launcher](https://github.com/Open-Wine-Components/umu-launcher)),
with vkd3d-proton turning its Direct3D 12 into Vulkan.

**This repository contains no game files, code or assets, and never will.**
You need your own copy of the game. It is not affiliated with or endorsed by
the game's developer or publisher, or by Microsoft.

## Status (2026-11-27)

| | |
|---|---|
| Cross-building `victory.exe` on Linux | ✅ works |
| Launcher, smoke and loader tests under Proton (no game files) | ✅ pass |
| The GPU helper shaders under Proton (indirect draws, masked patch, card compare, card fetch) | ✅ compiled with DXC, because Wine's own compiler refuses them; their tests pass on an RTX 3080 Ti |
| Running a title under Proton | ⚠️ boots, loads and draws the first scenes (thousands of draws a frame, 0.2–1 fps), and ends after 7–10 minutes at an 18 GB cap. On NVIDIA's 620 drivers a run can freeze the desktop (below). Needs Proton Experimental **and vkd3d-proton built from master** (below): the vkd3d-proton in current Proton releases crashes on some of the title's shaders |
| Performance measurements on Linux | none yet |

The details, with the commands that produced them, are in
[docs/kc-linux-status.md](docs/kc-linux-status.md). This README is updated
as the work moves on.

## Branches

| Branch | What it is |
|---|---|
| `kc/nvidia` | **Main work.** hyphennn's NVIDIA fork (the only public tree measured on a GeForce card) plus this fork's Linux work |
| `kc/linux` | The same Linux work on top of current upstream VIctory |
| `kc/hlsl-probe` | `kc/linux` plus the HLSL helper test |
| `main` | Upstream VIctory plus this fork's repository setup |

## Running under Proton

### What you need

- x86-64 Linux, a CPU with AVX2, and an NVIDIA card with the proprietary
  driver. vkd3d-proton must expose Shader Model 6.6 and mesh shaders.
- `umu-run` from [umu-launcher](https://github.com/Open-Wine-Components/umu-launcher#installation).
- **Proton Experimental** (Steam: install "Proton Experimental"), passed
  with `PROTONPATH`. UMU-Proton 11.0-2e's vkd3d-proton turns two of the
  title's compute shaders into invalid SPIR-V, and NVIDIA's driver crashes
  on it (the window just closes); Experimental's does not.
- **vkd3d-proton from master**, until a Proton release carries it. The
  shader converter inside Proton Experimental 2026-11-12 (dxil-spirv from
  2026-11-11) recurses forever on some of the title's largest shaders, and
  the title stops a little later. dxil-spirv fixed this on 2026-11-25. Build
  vkd3d-proton (needs meson, ninja, glslang, mingw-w64 and its `widl`):

  ```sh
  git clone --recursive https://github.com/HansKristian-Work/vkd3d-proton
  cd vkd3d-proton
  meson setup --cross-file build-win64.txt --buildtype release build.64
  ninja -C build.64
  ```

  and put `build.64/libs/d3d12/d3d12.dll` and
  `build.64/libs/d3d12core/d3d12core.dll` beside `victory.exe` (Wine loads
  DLLs from the executable's folder first), or pass their folder to
  `kc-run.sh --vkd3d`. The Proton log (`PROTON_LOG=1`) names the build that
  loaded.
- **Memory.** The title's own memory is about 8 GB, and the goal is
  12–14 GB for a whole run, as on a console. Today a run goes past 16–18
  GB, because victory keeps copies of title memory in the PC's memory for
  the GPU, and NVIDIA's Linux driver pins every page of those buffers
  when they are made (Windows pages out what nothing touches). This is
  being cut down (see the status doc). The first build of the title's
  pipelines also takes several GB at once: `VICTORY_PIPELINE_WORKERS=4`
  builds fewer at a time.
- **NVIDIA's 620 drivers can freeze the desktop.** After the driver logs
  `NVRM: intermapRegisterDmaMapping: Failed to insert new mapping node`
  (`journalctl -k`), its table of mappings is broken, and when the
  process exits the driver can loop holding its lock: the screen
  freezes and only a restart helps
  ([NVIDIA/open-gpu-kernel-modules#1540](https://github.com/NVIDIA/open-gpu-kernel-modules/issues/1540),
  fix in [#1542](https://github.com/NVIDIA/open-gpu-kernel-modules/pull/1542),
  not merged yet). Other Proton games hit it too; low memory and many
  buffers made and freed make it likelier. 615.x drivers did not freeze
  on it.
- **DXC**, Microsoft's open-source shader compiler: `dxcompiler.dll` and
  `dxil.dll` from `bin/x64` of a
  [DirectXShaderCompiler release](https://github.com/microsoft/DirectXShaderCompiler/releases).
  Put both in one folder and set `VICTORY_DXC` to that folder. Wine's own
  HLSL compiler refuses several shaders the GPU code compiles at runtime.
  Without DXC, indirect draws are dropped.
- Your own dump of the title, outside this repository.
- A Windows build of `victory.exe` (build it on Linux, below).

### Build `victory.exe` on Linux

Use rustup. The toolchain is pinned in `rust-toolchain.toml`. Also install
the x86-64 Linux UCRT build of
[LLVM-MinGW](https://github.com/mstorsjo/llvm-mingw/releases) and put its
`bin` first on `PATH`.

```sh
rustup target add x86_64-pc-windows-gnullvm
cargo build --locked --release -p victory --target x86_64-pc-windows-gnullvm
```

`scripts/kc-env.sh` sets all of this up when it is sourced
(`. scripts/kc-env.sh`). It finds LLVM-MinGW under `~/.local/opt` and can
use a rustup kept apart from a system Rust. Read it first, then adjust
it to your machine.

### Check without the game

```sh
bash tests/proton-launcher.sh   # the launcher's argument and path handling; no Wine
./tests/proton-smoke.sh         # a synthetic title under real Proton; expects exit 42
```

`proton-smoke.sh` runs the `victory.exe` already built, so build first.

### Run the title

```sh
export VICTORY_DXC=/path/to/dxc                                     # the folder with dxcompiler.dll
export PROTONPATH="$HOME/.local/share/Steam/steamapps/common/Proton - Experimental"
export WINEPREFIX="$HOME/.local/share/victory/proton-exp"           # its own prefix
export VICTORY_WAVE_WIDE=last                                       # NVIDIA's 32-lane waves
export VICTORY_PIPELINE_WORKERS=4 VICTORY_PIPELINE_COMPUTE_WAIT=0   # see "Memory" above; the status doc
./scripts/victory-proton.sh "/path/to/your/dump/title.exe"            # map and bind only
./scripts/victory-proton.sh "/path/to/your/dump/title.exe" --execute  # run it
```

- **Map and bind first.** Leave out `--execute` to check that the title maps
  and its imports bind, without running it.
- **Options.** Other `victory run` options are passed through, and
  arguments after `--` go to the title.
- **Prefix and cache.** The default Wine prefix is
  `~/.local/share/victory/proton`, and the shader caches go under
  `~/.cache/victory`. `WINEPREFIX`, `GAMEID`, `PROTONPATH` and `VICTORY_EXE`
  override the defaults. See [docs/proton-launcher.md](docs/proton-launcher.md).
- **Paths in `victory.toml`.** Copy `victory.example.toml` to `victory.toml`
  and write its paths in Windows form. Your files appear on drive `Z:`, for
  example `Z:\home\you\dumps\title`.
- **Don't run `winetricks d3dcompiler_47`.** That compiler is too old for
  these shaders and makes things worse. See the status doc.
- **No pad input with Steam running?** Steam Input keeps four empty virtual
  Xbox 360 pads that take every XInput slot. In the prefix, turn off
  winebus's hidraw and evdev paths (`DisableHidraw`=1, `DisableInput`=1 under
  `HKLM\System\CurrentControlSet\Services\winebus`) and set
  `SDL_GAMECONTROLLER_IGNORE_DEVICES=0x28de/0x11ff`, so SDL hands your own
  pad to slot 0. With no pad at all, the keyboard is one.
- **Recorded runs.** `scripts/kc-run.sh --profile linux-nvidia --mem-max 18G
  --watchdog TITLE.exe` runs the same with a memory cap, a freeze watchdog
  and a run folder outside the checkout (`victory.log`, GPU and memory
  readings, the crash log).

The launcher converts a Linux `VICTORY_DXC` path to the `Z:` path that
Wine needs. Runs so far reach the first scenes but do not last (see the
status table), so treat a run as a test, not a way to play.

## Development

```sh
. scripts/kc-env.sh
cargo xtask install-hooks   # the asset-policy check on every commit
cargo xtask policy          # the same check over the whole tree
```

- **Asset policy.** The pre-commit hook is an allowlist (see
  [docs/asset-policy.md](docs/asset-policy.md)). Nothing from a game may be
  committed.
- **Run logs.** Logs of real runs stay out of git.
- **Native Linux tests.** The ones that build natively, and the test of the
  runtime HLSL helpers under Proton, are in
  [docs/kc-linux-status.md](docs/kc-linux-status.md).
- **More docs.** [docs/linux-proton.md](docs/linux-proton.md) and
  [docs/arch-proton-rendering.md](docs/arch-proton-rendering.md) cover Linux
  further. For the run options, debugging variables and keyboard controls
  inherited from upstream and hyphennn, see
  [docs/hyphennn-readme.md](docs/hyphennn-readme.md).

## Credits

- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its
  contributors: the loader, runtime services, Direct3D 12 back end and shader
  recompiler that everything here is built on.
- **hyphennn ([fxceless/victory-fork](https://github.com/fxceless/victory-fork))**
  and the contributors to that fork: the NVIDIA work on `kc/nvidia`. Their
  history is kept with authors and dates, without `diagnostics/`. Their
  README, with its own credits (XCL by ivall as a reference, myokex,
  Joshua Odom and others), is [docs/hyphennn-readme.md](docs/hyphennn-readme.md).
- **[JMAN730/victory](https://github.com/JMAN730/victory)**: the Proton
  launcher, the smoke test, the Linux notes and the DXC fallback for the
  helper shaders (`dffc5924`). These are by Codex, SkippySteve, MiguVT,
  pablogonz12 and NiHuShu, and are picked or adapted with credit in each
  commit.
- Other community forks may be drawn on later, and are credited in the
  commits that do so.
- This fork's changes are developed with
  [Claude Code](https://claude.com/claude-code).

## License

GPL-3.0-or-later, as upstream. See [LICENSE](LICENSE).
