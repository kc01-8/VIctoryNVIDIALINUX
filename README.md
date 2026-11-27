# **NO COPYRIGHTED CONTENT PERMITTED. 100% CORRECTLY LICENSED CODE ONLY.**

## **ALL IN ACCORDANCE WITH THE LICENSES OF THE FORKED PROJECTS: GPL-3.0-OR-LATER ([VIctory](https://github.com/Marvisak/VIctory), [fxceless/victory-fork](https://github.com/fxceless/victory-fork), [JMAN730/victory](https://github.com/JMAN730/victory), [foxhound](https://github.com/readmet3xt-lab/foxhound), [trustno1-coder/VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA)).**

## **NO PULL REQUESTS WILL BE ACCEPTED WHICH DO NOT STRICTLY COMPLY WITH THESE LICENSES.**

---

# VIctoryNVIDIALINUX: the foxhound base

This branch, `kc/foxhound`, is
[trustno1-coder/VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA)
(on [readmet3xt-lab/foxhound](https://github.com/readmet3xt-lab/foxhound)),
the fastest VIctory tree on NVIDIA cards under Windows, with this fork's
work for **x86-64 Linux under Proton** carried onto it. VIctory is the
compatibility layer that runs an Xbox Series title on a PC from files you
supply.

`victory.exe` stays a Windows program. On Linux it runs under Proton
(through [umu-launcher](https://github.com/Open-Wine-Components/umu-launcher)),
with vkd3d-proton turning its Direct3D 12 into Vulkan.

**This repository contains no game files, code or assets, and never will.**
You need your own copy of the game. It is not affiliated with or endorsed by
the game's developer or publisher, or by Microsoft.

**This build downloads nothing.** trustno1's launcher fetched NVIDIA's DLSS
and frame generation files, AMD's FSR 3 files, Microsoft's Direct3D 12
runtime and a third-party mod; here those commands say where to put the
files and stop. Supply them yourself if you want them.

## Status (2026-11-27)

| | |
|---|---|
| Cross-building `victory.exe` on Linux (gnullvm) | ✅ builds |
| This fork's Linux work carried over | in progress (see the branch's history) |
| Running a title under Proton | not measured on this base yet; on `kc/nvidia` the title reaches its first scenes at 0.2–1 fps |

The goal: every shader made, then 60 frames a second. trustno1's tree runs
26–30 frames a second at 1440p on an RTX 5070 under Windows; hyphennn's
tree (`kc/nvidia`) 8–11.

## Branches

| Branch | What it is |
|---|---|
| `kc/foxhound` | **This branch.** trustno1's foxhound-based tree plus this fork's Linux work |
| `kc/nvidia` | hyphennn's NVIDIA fork plus this fork's Linux work, with the run history in its `docs/kc-linux-status.md` |
| `kc/linux` | The Linux work on top of current upstream VIctory |
| `main` | Upstream VIctory plus this fork's repository setup |

trustno1's own README, with its credits, settings menu and Windows
instructions, is [docs/README-trustno1.md](docs/README-trustno1.md);
foxhound's is [docs/README-foxhound.md](docs/README-foxhound.md).

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
- **The BCPack tables, which you must source yourself.** The title's
  streamed textures are packed with BCPack, and decoding needs the
  format's constant tables. They are not this project's work and its
  licence does not cover them, so this repository does not carry them (not
  in its history either). Get them from material you are entitled to use,
  as one file, `bcpack-tables.bin` (77,982 bytes, SHA-256 `1f1d4760…c36b`,
  in full in [docs/asset-policy.md](docs/asset-policy.md)), and put it
  beside `victory.exe` or set `VICTORY_BCPACK_TABLES` to it (a Linux path
  is turned into a Wine one by the launcher). Without it those textures
  stay empty.
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

- **[trustno1-coder/VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA)**
  and those who sent it pull requests: the base of this branch, with its
  history kept (the BCPack tables removed from every commit). Their own
  credits, for the many projects that tree was built from, are in
  [docs/README-trustno1.md](docs/README-trustno1.md).
- **[foxhound](https://github.com/readmet3xt-lab/foxhound)** by
  readmet3xt-lab: the base of trustno1's tree.
- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its
  contributors: the loader, runtime services, Direct3D 12 back end and
  shader recompiler.
- **hyphennn ([fxceless/victory-fork](https://github.com/fxceless/victory-fork))**
  and that fork's contributors, which foxhound built on, and which
  `kc/nvidia` is.
- **[JMAN730/victory](https://github.com/JMAN730/victory)**: the Proton
  launcher, the smoke test, the Linux notes and the DXC fallback for the
  helper shaders, by Codex, SkippySteve, MiguVT, pablogonz12 and NiHuShu.
- This fork's changes are developed with
  [Claude Code](https://claude.com/claude-code).

## License

GPL-3.0-or-later, as VIctory, foxhound and trustno1's tree. See
[LICENSE](LICENSE).
