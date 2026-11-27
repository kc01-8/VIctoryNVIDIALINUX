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

## Status (2026-11-26)

| | |
|---|---|
| Cross-building `victory.exe` on Linux | ✅ works |
| Launcher, smoke and loader tests under Proton (no game files) | ✅ pass |
| Running a title under Proton | ❌ **not yet usable.** Wine's own HLSL compiler refuses five helper shaders the GPU code compiles at runtime (indirect draws, the masked patch, the card compare). Being fixed now |
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
  On first launch it fetches UMU-Proton and the Steam Linux Runtime.
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

Until the helper-shader fix lands, a run under Proton loses indirect draws
and several GPU paths, so treat it as a test, not a way to play.

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
  launcher, the smoke test and the Linux notes. These are by Codex,
  SkippySteve, MiguVT, pablogonz12 and NiHuShu, and are picked with credit in
  each commit.
- Other community forks may be drawn on later, and are credited in the
  commits that do so.
- This fork's changes are developed with
  [Claude Code](https://claude.com/claude-code).

## License

GPL-3.0-or-later, as upstream. See [LICENSE](LICENSE).
