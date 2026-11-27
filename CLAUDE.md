# Notes for Claude Code

This repo is a public fork of [VIctory](https://github.com/Marvisak/VIctory). It focuses on NVIDIA GPUs and Linux (Proton/UMU).

## Rules

1. **Nothing proprietary in git.** No game files, dumps, extracted shaders, screenshots, unscrubbed logs, built binaries, or pipeline caches. `cargo xtask policy` is authoritative. Install the hook with `cargo xtask install-hooks`, and never bypass it with `--no-verify`. See `docs/asset-policy.md`.
2. **No network paths** in the emulated title's runtime.
3. **GPL-3.0-or-later.** Keep upstream history intact. Credit every change taken from another fork in the commit message: `after <fork> <sha>` or `PR #n by <user>`.
4. **Stay title-agnostic and personal-data-free.** Don't name specific titles, and don't commit local paths, machine details, or byte patterns copied from title binaries. Scripts use `$HOME`/`$XDG_*`. Machine-specific values go in the untracked `local.env`.
5. **Measure before you claim.** A performance change gets an on/off measurement before it becomes a default.
6. **Ask before** anything destructive or global: package removal, prefix wipes, force-pushes, driver or kernel changes.

## Local notes

Maintainer-only notes live in `private.md`, which is gitignored. If the file doesn't exist, the import below is skipped.

@private.md
