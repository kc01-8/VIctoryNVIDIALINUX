# **NO COPYRIGHTED CONTENT PERMITTED. 100% CORRECTLY LICENSED CODE ONLY.**

## **ALL IN ACCORDANCE WITH THE LICENSES OF THE FORKED PROJECTS: GPL-3.0-OR-LATER ([VIctory](https://github.com/Marvisak/VIctory), [fxceless/victory-fork](https://github.com/fxceless/victory-fork), [JMAN730/victory](https://github.com/JMAN730/victory), [foxhound](https://github.com/readmet3xt-lab/foxhound), [trustno1-coder/VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA)).**

## **NO PULL REQUESTS WILL BE ACCEPTED WHICH DO NOT STRICTLY COMPLY WITH THESE LICENSES.**

---

<div align="center">

# VIctoryNVIDIALINUX

**Running an Xbox Series title on an NVIDIA GeForce card under Linux and Proton.**

A fork of [VIctory](https://github.com/Marvisak/VIctory), built on
[trustno1-coder/VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA).
By [kc01-8](https://github.com/kc01-8), developed with
[Claude Code](https://claude.com/claude-code). GPL-3.0-or-later.

[Status](#status-at-a-glance) ·
[Progress](#progress-so-far) ·
[What this fork adds](#what-this-fork-adds-for-linux) ·
[Next steps](#whats-next) ·
[Run it](#running-under-proton) ·
[Continue the work](#for-agents-and-contributors) ·
[Credits](#credits)

</div>

> **No game files, code or assets are in this repository, and none ever
> will be.** You need your own copy of the game. This project is not
> affiliated with or endorsed by the game's developer or publisher, or by
> Microsoft. **It downloads nothing**: no DLSS, FSR, Direct3D or mod files
> are fetched; supply them yourself if you want them. **The BCPack tables
> must be sourced by you** (see [What you need](#what-you-need)).

## Status at a glance

On the test PC (RTX 3080 Ti 12 GB, NVIDIA 620.73.04, 32 GB RAM, Proton
Experimental with vkd3d-proton from master), branch `kc/foxhound`,
2026-11-28:

| | |
|---|---|
| **Boots and plays** | ✅ Loads into the 3D world and stays there until stopped (runs of 6–9 minutes; one of 20 minutes on `kc/nvidia`) |
| **Looks** | ✅ Most of the scene is drawn with its lighting and materials; by the maintainer's eye already ahead of the other public Linux attempts |
| **Shaders** | ✅ **2,533 of the 2,534 shaders** the title asked for in a run translate (**99.96 %**); all **2,185** host pipelines made, **none failed** |
| **What reaches the screen** | ⚠️ Of about 2,440 draws and dispatches a frame, **~69 % are drawn**; ~19 % are ray-tracing work left out on purpose; ~12 % wait for pipelines that are still compiling, or are the one program that does not translate |
| **Frame rate** | ⚠️ **8–14 fps** at 1,400–3,700 draws a frame at best (run 41); 2–6 fps with the memory-saving settings (run 44). Goal: **60** |
| **Memory** | ✅ 12–14 GB that cannot be reclaimed with title memory read in place (the goal: 12–14 GB, as on a console; runs 49–51: 11.8–12.7 GB); 16–17 GB in the faster mode |
| **Overlay** | ✅ A numbers panel (`VICTORY_PANEL=1` or F3): frame rate, frame-time and FPS graphs, every core, the card, VRAM and RAM, a frame's work, and a tape of the shaders and pipelines being made |
| **Builds on Linux** | ✅ `victory.exe` cross-builds (gnullvm); unit tests run under Proton's Wine |

Upstream: this fork's DXC fallback for Wine and its Linux notes are pull
requests to trustno1's tree
([#925](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/925),
[#926](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/926)).

Every number here comes from a recorded run; each run is in
[docs/kc-linux-status.md](https://github.com/kc01-8/VIctoryNVIDIALINUX/blob/kc/foxhound/docs/kc-linux-status.md)
(runs 33–44) and in `kc/nvidia`'s
[status doc](https://github.com/kc01-8/VIctoryNVIDIALINUX/blob/kc/nvidia/docs/kc-linux-status.md)
(runs 3–32).

## Branches

| Branch | What it is |
|---|---|
| [`kc/foxhound`](https://github.com/kc01-8/VIctoryNVIDIALINUX/tree/kc/foxhound) | **The main work.** trustno1's tree (foxhound base), the fastest VIctory tree on NVIDIA under Windows, with this fork's Linux work on it |
| [`kc/nvidia`](https://github.com/kc01-8/VIctoryNVIDIALINUX/tree/kc/nvidia) | The first line: hyphennn's NVIDIA fork with the same Linux work, and the history of runs 3–32 |
| [`kc/linux`](https://github.com/kc01-8/VIctoryNVIDIALINUX/tree/kc/linux) | The Linux work on top of upstream VIctory |
| [`kc/hlsl-probe`](https://github.com/kc01-8/VIctoryNVIDIALINUX/tree/kc/hlsl-probe) | `kc/linux` plus the HLSL helper test |
| `main` | Upstream VIctory plus this README |

---

## Progress so far

From the first Linux commit to the 3D world at up to 14 fps took **two
days** (2026-11-26 to 2026-11-27) and 44 recorded runs.

| When | Where | What changed | What it did |
|---|---|---|---|
| 11-26 | `kc/linux` | `victory.exe` cross-built on Linux; the Proton launcher, a smoke test, DXC for the helper shaders Wine's compiler refuses | Launcher and smoke tests pass under Proton |
| 11-26 – 11-27 | `kc/nvidia`, runs 3–24 | vkd3d-proton from master (the shader converter in Proton recursed forever on the title's largest shaders); the empty pipeline library; fewer pipeline workers | From crashes at start to the first 3D scenes: up to 6,000 draws a frame, **0.1–0.8 fps**, killed at an 18 GB cap |
| 11-27 | `kc/nvidia`, runs 25–32 | Title memory read in place instead of copied (`VICTORY_INPLACE`, `VICTORY_INPLACE_VIEWS`); the title's progress watchdog disarmed | Memory from the 18 GB cap to **12.2–13.1 GB**; a full **20-minute** run |
| 11-27 | `kc/foxhound`, runs 33–36 | Moved to trustno1's tree, the Linux work carried over; every helper shader compiled at run time falls back to DXC | **1.6 → 5.6 fps** at 1,600–3,100 draws, 12.9 GB |
| 11-27 | runs 37–43 | The sampling profiler found to crash the title; write faults without the guards' lock; the wineserver's requests traced; the root signatures Wine's compiler drops sent to DXC | **8.4–14 fps** at 1,400–3,700 draws; the depth planes, the scatter, the upload check, the tiles' look and keeping the card awake all working |
| 11-28 | run 44 | 48 KiB of shared memory for thread groups; a CPU profile with `perf` | NVIDIA's shader compiler is 46 % of CPU samples three minutes in: compiled pipelines are not kept between runs |
| 11-28 | runs 45–51 | NVIDIA's shader cache kept between runs; the title borderless at the screen's size; the numbers panel | a second run has half the pipelines left to compile at 90 s; the picture no longer resampled; 11.8–12.7 GB |
| 11-28 | runs 52–56 | trustno1's 64 newer commits, foxhound 0.2.0-pre.4's settings and hyphennn's static-sampler order brought over; write faults under the guards' lock again | **every pipeline made by 90 seconds** on a second start (NVIDIA's compiler had been 41–46 % of the CPU in play); no more early stops (all five came of unlocked write faults); 12.1–12.3 GB |

## What this fork adds for Linux

All of it is in `kc/foxhound`, each with its own commit and measurements.

- **Proton launcher and run harness.**
  - `scripts/victory-proton.sh`: runs `victory.exe` through umu-launcher and converts Linux paths to Wine ones.
  - `scripts/kc-run.sh`: a run with a memory cap (`--mem-max`, a systemd scope, no swap), a freeze watchdog, a vkd3d-proton build of your choice (`--vkd3d`) and a run folder with the log and readings.
  - `profiles/*.env`: the switches for a run. A switch already in the environment wins over the profile's line.
- **Helper shaders under Wine.** victory compiles its own helper shaders at run time, and Wine's built-in HLSL compiler gets them wrong in two ways:
  - It refuses many of them.
  - It accepts `[RootSignature(...)]` and leaves it out of the code, which vkd3d-proton then rejects with `E_FAIL`.
  - Both now go to DXC (`crates/victory-gpu/src/hlsl.rs`). This brought back the depth planes, the indirect clamp, the card compare, the ambient light's stand-in, the scatter, the upload check, the tiles' look and keeping the card awake.
- **Shared memory past D3D12's 32 KiB.** `VICTORY_HOST_LDS_KB` gives the translator what Vulkan allows: 48 KiB on NVIDIA. Translated code is not validated, and vkd3d-proton passes it on.
- **Title memory read in place.** The GPU reads it where it is, instead of through copies:
  - `VICTORY_INPLACE`: private memory, opened as a heap from its address.
  - `VICTORY_INPLACE_VIEWS`: the page pool, through views of our own.

  This matters because NVIDIA's Linux driver pins every page of a buffer it is given, and Windows does not. In-place reading is what brought a run under 14 GB.
- **The title's progress watchdog disarmed** (`VICTORY_NO_WATCHDOG`, checked against the build's hash). Under Wine the title is slow enough for it to fire.
- **The empty pipeline library.** A null-pointer fix in the pipeline library, so pipelines are made early on a second start.
- **The BCPack tables supplied by the user.** They are no longer in the repository (`VICTORY_BCPACK_TABLES`, or beside `victory.exe`).
- **NVIDIA's shader cache under umu.** The launcher gives the driver a cache folder of its own (`~/.cache/victory/nvidia`) and a size it takes (under 4 GiB), so compiled pipelines are kept between runs.
- **The title borderless at the screen's size** (`VICTORY_FULLSCREEN=1` in the profile), so the picture is shown pixel for pixel.
- **A numbers panel** (`VICTORY_PANEL=1`, F3): drawn on the CPU in the system's own fonts through GDI (no font ships with it), copied onto the picture. Under Wine its RAM is the process's own proportional anonymous and shared memory from Linux (`/proc/<pid>/smaps_rollup`; `Z:\proc\self` is the wineserver's).
- **Pipelines compiled once.** NVIDIA's cache only answers for a pipeline whose static samplers sit where they sat before; hyphennn's kept order (`pipeline/sampler_order.rs`, picked from fxceless/victory-fork) makes them, and a second start has every pipeline made within 90 seconds.
- **No downloads.** trustno1's launcher fetched DLSS, FSR, the Direct3D 12 runtime and a mod; here those commands say where to put the files and stop.

## Where it stands, in detail

**Shaders and pipelines (run 44).**
- The title knows about 19,100 shaders in 89,000 pipelines.
- Of those it used in a 7-minute run, 2,533 shaders translated and 1 did not. 2,185 host pipelines were made, and none failed.
- What a frame still leaves out:
  - **Ray tracing** (about 450–570 dispatches a frame). This tree leaves it out unless its structure builder is on (`VICTORY_BVH_BUILDER=1`, called slow and broken on this base by trustno1), and draws an ambient stand-in for the light.
  - **Pipelines still compiling** (about 300 draws a frame, 3–5 minutes in). See [next](#whats-next).
  - **One compute program**, `6b82bc7d22016cb5`: the triangle clip's meshlet collect, groups of 1,024. It has no 32-lane form yet, so its 58–65 indirect dispatches a frame are dropped.
  - **The title's upscaler's precompiled pipelines**, through device method `0x218`, `DeserializeComputePipelineStateX`. No tree implements it, so the title goes without them.

**Where a frame goes.** The card is 28–39 % busy, so the frame is the CPU's. `perf` over 20 seconds of play (run 44):

| Share of samples | Where |
|---|---|
| 29 % | NVIDIA's shader compiler (`libnvidia-gpucomp`) on victory's pipeline workers |
| 17 % | The same compiler on the title's own threads |
| 10 % | The kernel on the title's threads, mostly system calls (Wine's sync objects) |
| 9 % | The title's and victory's code |

The prefix's shader cache is empty: compiled pipelines are not reused from one run to the next.

**The page pool: views or copies.** Under Wine every view mapped or unmapped is a request to the wineserver, a single thread for the whole prefix.
- **With views:** the title's remaps kept the wineserver at 81 % of a core, and the title's threads waited behind it.
- **As copies:** the wineserver dropped to 30 % and the frame rate doubled, but the driver pins the copies (4.4 GB more).
- **Neither cheaper option works.** Copying only the cells that churn did not help, because most cells churn. The card's memory is full in play (11.4–11.9 GB of 12), so the copies cannot move there either.

**Known problems.**
- **NVIDIA's 620 drivers can freeze the desktop.** After `NVRM: intermapRegisterDmaMapping: Failed to insert new mapping node` (`journalctl -k`), the driver can loop holding its lock when a process exits ([open-gpu-kernel-modules#1540](https://github.com/NVIDIA/open-gpu-kernel-modules/issues/1540), fix in [#1542](https://github.com/NVIDIA/open-gpu-kernel-modules/pull/1542)). Other Proton games hit it too. Low memory and many buffers made and freed make it likelier.
- **The built-in sampling profiler crashes the title under Wine** (`VICTORY_PROFILE`): it suspends the title's threads to read their stacks. Use `perf`.
- **On `kc/nvidia`, the title stopped after 10–17 minutes** with an object pool full, while its opening scene was still streaming.

## What's next

Ranked by what they should give. Each item names where to start.

1. **The title's threads' system calls.** With pipelines compiled ahead, a quarter of the CPU's samples are the kernel on the title's threads, mostly entering and leaving system calls (Wine's sync objects, `getrusage`), with the processor's speculation mitigations on every return. Which calls, and which can stay in user space.
2. **The title's own frame loop.** In some stretches its main thread spins in a lookup of its pools' object handles until another thread adds one (`VICTORY_HANDLE_TABLE=1` reports the table); what that thread waits on.
3. **The page pool without the wineserver or pinned copies.** Fewer, larger views; or keeping the views and remapping only what the title remaps (`crates/victory-gpu/src/inplace.rs`, `shadow.rs`).
4. **Ray tracing**, with `VICTORY_BVH_BUILDER=1` as the starting point.
5. **The meshlet program.** Run with `VICTORY_WAVE_WHY=1` to see what stops it now that 48 KiB fit (`crates/victory-shader/src/emit/lower.rs`).
6. **Device method `0x218`**: precompiled native modules. [JMAN730's tree](https://github.com/JMAN730/victory) already parses their pieces.
7. **Wine's `VirtualProtect`** costs about 0.44 ms a block for the write guards (`crates/victory-gpu/src/watch.rs`).

---

## Running under Proton

### What you need

- x86-64 Linux, a CPU with AVX2, and an NVIDIA card with the proprietary
  driver. vkd3d-proton must expose Shader Model 6.6 and mesh shaders.
- `umu-run` from [umu-launcher](https://github.com/Open-Wine-Components/umu-launcher#installation).
- **Proton Experimental** (in Steam, install "Proton Experimental"), passed
  with `PROTONPATH`. UMU-Proton 11.0-2e's vkd3d-proton turns two of the
  title's compute shaders into invalid SPIR-V, and NVIDIA's driver crashes
  on it.
- **vkd3d-proton from master**, until a Proton release carries it. The
  shader converter in Proton Experimental 2026-11-12 recurses forever on
  some of the title's largest shaders; dxil-spirv fixed this on
  2026-11-25. Build it (needs meson, ninja, glslang, mingw-w64 and its
  `widl`):

  ```sh
  git clone --recursive https://github.com/HansKristian-Work/vkd3d-proton
  cd vkd3d-proton
  meson setup --cross-file build-win64.txt --buildtype release build.64
  ninja -C build.64
  ```

  Put `build.64/libs/d3d12/d3d12.dll` and `build.64/libs/d3d12core/d3d12core.dll`
  beside `victory.exe`, or pass their folder to `kc-run.sh --vkd3d`.
- **DXC**, Microsoft's open-source shader compiler: `dxcompiler.dll` and
  `dxil.dll` from `bin/x64` of a
  [DirectXShaderCompiler release](https://github.com/microsoft/DirectXShaderCompiler/releases),
  in one folder named by `VICTORY_DXC`. Without it many helper passes are
  off and indirect draws are dropped.
- **The BCPack tables, which you must source yourself.** Decoding the
  title's BCPack-packed textures needs the format's constant tables. They
  are not this project's work, its licence does not cover them, and this
  repository does not carry them (not in its history either). Supply them
  from material you are entitled to use as one file, `bcpack-tables.bin`
  (77,982 bytes, SHA-256 `1f1d4760…c36b`, in full in
  [docs/asset-policy.md](https://github.com/kc01-8/VIctoryNVIDIALINUX/blob/kc/foxhound/docs/asset-policy.md)),
  beside `victory.exe` or named by `VICTORY_BCPACK_TABLES`. Without them
  those textures stay empty.
- **Memory.** Plan for 12–14 GB with the `linux-foxhound` profile, more in
  the faster mode, and more while the title's pipelines are first built
  (`VICTORY_PIPELINE_WORKERS=4` builds fewer at a time).
- Your own dump of the title, outside this repository, and a Windows build
  of `victory.exe` (below).

### Build `victory.exe` on Linux

Use rustup; the toolchain is pinned in `rust-toolchain.toml`. Install the
x86-64 Linux UCRT build of
[LLVM-MinGW](https://github.com/mstorsjo/llvm-mingw/releases) and put its
`bin` first on `PATH`.

```sh
rustup target add x86_64-pc-windows-gnullvm
cargo build --locked --release -p victory --target x86_64-pc-windows-gnullvm
```

`scripts/kc-env.sh` sets this up when sourced (`. scripts/kc-env.sh`). Read
it first and adjust it to your machine.

### Check without the game

```sh
bash tests/proton-launcher.sh   # the launcher's argument and path handling; no Wine
./tests/proton-smoke.sh         # a synthetic title under real Proton; expects exit 42
```

### Run the title

```sh
export VICTORY_DXC=/path/to/dxc
export PROTONPATH="$HOME/.local/share/Steam/steamapps/common/Proton - Experimental"
export WINEPREFIX="$HOME/.local/share/victory/proton-exp"
export VICTORY_PIPELINE_WORKERS=4

# a recorded run: memory cap, freeze watchdog, run folder, this fork's switches
scripts/kc-run.sh --profile linux-foxhound --mem-max 18G --watchdog \
    --vkd3d /path/to/vkd3d-proton/build "/path/to/your/dump/title.exe"

# the same switches by hand
set -a; . profiles/linux-foxhound.env; set +a
./scripts/victory-proton.sh "/path/to/your/dump/title.exe"             # map and bind only
./scripts/victory-proton.sh "/path/to/your/dump/title.exe" --execute   # run it
```

- **The numbers panel:** `VICTORY_PANEL=1`, or F3 in play. `VICTORY_PANEL_BLEND=1` blends it with a translucent ground (off by default: two runs with it stopped early).
- **Faster but heavier:** `VICTORY_INPLACE_VIEWS=0` (page-pool windows as
  copies: the best frame rate so far, 16–17 GB).
- **Paths in `victory.toml`** are in Windows form; your files are on drive
  `Z:` (for example `Z:\home\you\dumps\title`).
- **Don't run `winetricks d3dcompiler_47`**: it is too old for these
  shaders.
- **No pad input with Steam running?** Steam Input's four virtual pads take
  every XInput slot. In the prefix set `DisableHidraw`=1 and
  `DisableInput`=1 under `HKLM\System\CurrentControlSet\Services\winebus`,
  and `SDL_GAMECONTROLLER_IGNORE_DEVICES=0x28de/0x11ff`.
- Treat a run as a test, not a way to play, for now.

---

## For agents and contributors

How this fork works, so that you can pick it up where it is.

**Rules that are not negotiable.**
- **Nothing from the title, ever.** That means no dumps, shaders, shader dumps, logs of real runs or screenshots in git. The pre-commit hook is an allowlist ([asset policy](https://github.com/kc01-8/VIctoryNVIDIALINUX/blob/kc/foxhound/docs/asset-policy.md)), and `cargo xtask policy` checks the whole tree. The title's name stays out of code, comments and commit messages.
- **GPL-3.0-or-later.** Code taken from another fork keeps its author in the commit and says where it came from.
- **No network path.** The title stays offline, and nothing is downloaded at run time.

**Working on it.**
- **Build and tests.** `. scripts/kc-env.sh` sets up the cross-build. Unit tests run under Proton's own Wine binary in the run prefix, while no run is live:

  ```sh
  exe=$(cargo test --no-run -p victory-gpu --features diagnostics --target x86_64-pc-windows-gnullvm 2>&1 | grep -oE '\(.*victory_gpu-[0-9a-f]+\.exe\)' | tr -d '()')
  WINEPREFIX=~/.local/share/victory/proton-exp ".../Proton - Experimental/files/bin/wine" "$exe" <filter>
  ```

  `victory-shader` tests run natively: `cargo test -p victory-shader --lib`.
- **Don't run `cargo fmt` on `victory-gpu`.** The crate is not rustfmt-clean, and fmt rewrites dozens of files.
- **Measure, then claim.** Every change here came with a run, recorded in the status doc with what changed and what it did. A run folder (`~/.local/share/victory/runs/<UTC>-<commit>-<profile>/`) holds:
  - `victory.log`;
  - `mem-cg.csv`: the run's memory each second. Anonymous + shmem + kernel is what cannot be reclaimed, and the kernel part is the driver's pins;
  - `gpu.csv`: `nvidia-smi` each second;
  - `meta.txt`: the commit, the profile and any overrides.
- **Read the logs.** victory prints a line every 10 seconds on frame rate, draws, dropped work, copies, memory and title memory in place.
- **Profile with `perf`**, not with `VICTORY_PROFILE`. For example, `perf record -F 499 -p <pid of victory.exe> -- sleep 20`, then `perf report --sort comm,dso`. victory's own frames show as `[JIT]` and can be named with `llvm-symbolizer` against the diagnostics build.
- **Mind the driver.** On NVIDIA's 620 drivers, keep runs under a memory cap. Watch `journalctl -k` for the mapping error above, and restart before more runs if it shows.

**Credit.** If you build on this work, keep the GPL notices and credit
**VIctoryNVIDIALINUX by [kc01-8](https://github.com/kc01-8)** with the
projects below.

## Credits

- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its
  contributors: the loader, runtime services, Direct3D 12 back end and
  shader recompiler that all of this is built on.
- **[trustno1-coder/VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA)**
  and those who sent it pull requests: the base of `kc/foxhound`, with its
  history kept. Their own credits are in
  [docs/README-trustno1.md](https://github.com/kc01-8/VIctoryNVIDIALINUX/blob/kc/foxhound/docs/README-trustno1.md).
- **[foxhound](https://github.com/readmet3xt-lab/foxhound)** by
  readmet3xt-lab: the base of trustno1's tree.
- **hyphennn ([fxceless/victory-fork](https://github.com/fxceless/victory-fork))**
  and that fork's contributors: the NVIDIA work foxhound built on, and the
  base of `kc/nvidia`.
- **[JMAN730/victory](https://github.com/JMAN730/victory)**: the Proton
  launcher, the smoke test, the Linux notes and the DXC fallback for the
  helper shaders, by Codex, SkippySteve, MiguVT, pablogonz12 and NiHuShu;
  and the name of device method `0x218`.
- **[vkd3d-proton](https://github.com/HansKristian-Work/vkd3d-proton)**,
  **[Proton](https://github.com/ValveSoftware/Proton)** and
  **[umu-launcher](https://github.com/Open-Wine-Components/umu-launcher)**,
  which make Direct3D 12 on Linux possible at all.
- **This fork, VIctoryNVIDIALINUX:** [kc01-8](https://github.com/kc01-8),
  with [Claude Code](https://claude.com/claude-code) (Claude Opus 5.5,
  Anthropic).

## License

GPL-3.0-or-later, as VIctory, foxhound and trustno1's tree. See
[LICENSE](LICENSE).
