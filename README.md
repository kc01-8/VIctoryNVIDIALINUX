# **NO COPYRIGHTED CONTENT PERMITTED. 100% CORRECTLY LICENSED CODE ONLY.**

## **ALL IN ACCORDANCE WITH THE LICENSES OF THE FORKED PROJECTS: GPL-3.0-OR-LATER ([VIctory](https://github.com/Marvisak/VIctory), [fxceless/victory-fork](https://github.com/fxceless/victory-fork), [JMAN730/victory](https://github.com/JMAN730/victory), [foxhound](https://github.com/readmet3xt-lab/foxhound), [trustno1-coder/VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA)).**

## **NO PULL REQUESTS WILL BE ACCEPTED WHICH DO NOT STRICTLY COMPLY WITH THESE LICENSES.**

---

# VIctoryNVIDIALINUX

A fork of [VIctory](https://github.com/Marvisak/VIctory), the compatibility
layer that runs an Xbox Series title on a PC from files you supply, aimed at
one setup: **an NVIDIA GeForce card on x86-64 Linux, under Proton**.

**This repository contains no game files, code or assets, and never will.**
You need your own copy of the game. It is not affiliated with or endorsed by
the game's developer or publisher, or by Microsoft.

**The BCPack tables are not in this repository and must be sourced by the
user.** Decoding the title's BCPack-packed textures needs the format's
constant tables, which are not this project's work and which its licence
does not cover; they were removed from this repository and its whole
history. Supply them yourself as `bcpack-tables.bin` (see
`docs/asset-policy.md` on `kc/nvidia` for its size and SHA-256 and where it
goes).

## Branches

The work is not on this branch. How to build and run under Proton, and where
things stand, are in the README of `kc/nvidia`.

| Branch | What it is |
|---|---|
| [`kc/nvidia`](../../tree/kc/nvidia) | **Main work.** hyphennn's NVIDIA fork plus this fork's Linux work: the Proton launcher, run scripts, memory and speed fixes for Proton |
| [`kc/linux`](../../tree/kc/linux) | The same Linux work on top of current upstream VIctory |
| [`kc/hlsl-probe`](../../tree/kc/hlsl-probe) | `kc/linux` plus the HLSL helper test |
| `main` | Upstream VIctory plus this fork's repository setup (this page) |

Upstream's own README, with its commands and debugging notes, is at
[Marvisak/VIctory](https://github.com/Marvisak/VIctory#readme).

## Credits

- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its
  contributors: the loader, runtime services, Direct3D 12 back end and shader
  recompiler that everything here is built on.
- **hyphennn ([fxceless/victory-fork](https://github.com/fxceless/victory-fork))**
  and the contributors to that fork: the NVIDIA work on `kc/nvidia`.
- **[JMAN730/victory](https://github.com/JMAN730/victory)**: the Proton
  launcher, the smoke test, the Linux notes and the DXC fallback for the
  helper shaders, by Codex, SkippySteve, MiguVT, pablogonz12 and NiHuShu,
  picked or adapted with credit in each commit.
- Other community forks drawn on are credited in the commits that do so.
- This fork's changes are developed with
  [Claude Code](https://claude.com/claude-code).

## License

GPL-3.0-or-later, as upstream. See [LICENSE](LICENSE).
