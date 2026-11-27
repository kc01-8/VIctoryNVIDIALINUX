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

## Title runs under Proton (2026-11-27)

RTX 3080 Ti (12 GB, driver 620.73.04), 32 GB RAM, `scripts/kc-run.sh
--mem-max 18G --watchdog`, HUD off, diagnostics build from run 14 on, Enter
pressed once 18 s in (`VICTORY_KEYS=18:Enter`) from run 20 on. Every run
maps the title, binds all imports (`missing=0`) and reaches the menus. No
NVRM Xid in any run, but NVIDIA's kernel driver logs mapping errors in
most runs from 22 on (run 26, below).

| Runs | Proton | What ended them |
|---|---|---|
| 3, 5, 6 | UMU-Proton 11.0-2e | NVIDIA's driver crashed inside `vkCreateComputePipelines` ("Exception 0xc0000005 / 0xc000001d in Unix call"), and Wine ended the process (exit 3, reported as 247): compute shaders `4e296b1885c37b29`, then `8bb272e763f76148` |
| 7, 8 | UMU-Proton 11.0-2e, both denied (`VICTORY_DENY_PIPELINES`) | no driver crash; the title stopped drawing and its own watchdog aborted it about 30 s later (its deliberate write to 0, `+0x23cf683`; see upstream `ff8694a6`) |
| 10, 11 | Proton Experimental 2026-11-12 | no driver crash; the same abort about 3 minutes in, while loading. In run 11, `02384a60fda6f2d0` and `369cbe906ea72764` were refused just before (below) |
| 12–19 | Proton Experimental 2026-11-12 | the same abort: a pipeline build overflowed its thread's stack, first on 1–2 MiB stacks, then on 64 MiB inside vkd3d-proton's shader converter (below) |
| 20, 21 | Experimental + vkd3d-proton master | no crash; frames stop at the first big load while the title waits for compute pipelines being made, and the watchdog fires (below) |
| 22 | as 20, the pipeline library fixed, `VICTORY_PIPELINE_COMPUTE_WAIT=0` | through the load into the first 3D scenes (up to 6,000 draws a frame, under 1 fps); killed by the kernel at the 18 GB cap 3m30s in, with 12 pipeline workers |
| 23 | as 22, `VICTORY_PIPELINE_WORKERS=4`, the library from run 22 | in the 3D scenes 2m25s in and still drawing (0.2–0.5 fps, peak 15.9 GB) when `kc-run.sh`'s own watchdog killed it at 5 min: it watched `run.log`, which stops growing once the log goes to `victory.log` (fixed) |
| 24 | as 23, the watchdog fixed | every known pipeline made 1m20s in, before the load; in the 3D scenes from 2 min at 0.1–0.8 fps, at the 18 GB cap from about 2m20s; killed by the kernel at the cap 9m30s in (below) |
| 25 | as 24, `VICTORY_INPLACE=1 VICTORY_INPLACE_PROBE=1` | killed by the kernel at the cap 7m10s in; 80 windows (380 MB) in place, the rest could not be (below) |
| 26 | as 25 without the probe, `VICTORY_WINDOW_KB=512 VICTORY_WINDOW_TAILS=1` | peak 16.2 GB, under the cap; froze the desktop 9m44s in: NVIDIA's driver looped holding its global lock while the title's process exited, after many failed GPU mappings (a 620.x driver bug, below); needed a reboot |
| 27 | as 25 on the 4 MiB grid, `VICTORY_INPLACE_VIEWS=1` (windows over the page pool in place) | peak 15.0 GB, no driver errors; 1,461 windows (6.6 GB of title memory) read in place; the title's progress watchdog ended it 5m30s in (below) |
| 28 | as 27, the watchdog hook (`VICTORY_NO_WATCHDOG=1`, in the profile), pool windows let go only for another page | ran the full 20 minutes; at most 13.1 GB that could not be reclaimed (peak 17.6 GB with file cache), no driver errors; the title stopped itself at 16m43s with an object pool full (below) |
| 29 | as 28, pool windows let go only where something read them | stopped by hand at 8 min: at most 12.4 GB that could not be reclaimed, no driver errors; a third fewer windows let go |
| 30 | as 29, slots that show another copy made again at once | the card filled to 99.8 % of a larger budget and 2.4–3.9 GB of its data went to the PC's memory: at most 15.8 GB that could not be reclaimed, at the cap from 8 min, and 23 failed driver mapping inserts there; the title stopped itself at 13m7s on the same break as run 28 (`+0x63966f`), the same pool at 44,546 of 45,000 |
| 31 | as 30, `VICTORY_TEXTURE_ROOM_PERCENT=85` | the card held at about 96 % of its budget, the driver's memory at 0.1 GB, at most 12.2 GB that could not be reclaimed; 50+ driver mapping errors 7 minutes in with no memory pressure; the title stopped itself on its full pool at about 13 minutes; no freeze |
| 32 | as 31, in-place windows indexed by page, disk writes through one view of each section, `VICTORY_TEXTURE_NO_WAIT=1` | the usual frame at 3 minutes 833 ms (about 5 s in run 31), texture refreshes waiting 0 ms (2.8–3.6 s), disk-write views 0 ms (39 s of the workers' time); later frames 1.2–2.9 s, nearly all of it remapping (below); ran the full 20 minutes |

**The driver crash, reproduced outside the title.** The saved shaders
(`VICTORY_SHADER_SAVE`, kept out of the checkout), built alone with
`victory translate <folder> --pipelines`, crash the same way. Their
SPIR-V from UMU-Proton 11.0-2e's vkd3d-proton (build `212991fc`) fails
`spirv-val`: one function ends in an unterminated block, the other reuses
a merge block. Proton Experimental's (build `2576c0b4`) passes, from the
same DXIL, and the driver builds both. So: Proton Experimental.

**Wave64 programs on a 32-lane card.** vkd3d refuses a pipeline whose
shader requires `WaveSize(64)` ("supported range is [32, 32]"). With
`VICTORY_WAVE_WIDE=last` (profile `linux-nvidia`), `369cbe906ea72764`
builds (checked offline with `VICTORY_HOST_MAX_WAVE=32`, which the runtime
sets itself from the card); `02384a60fda6f2d0` is still refused (LDS in a
group of 128, the mask as data, `v_mbcnt`, `v_readlane`). hyphennn's
Windows runs on an RTX 4070 refused both too and went on, so these alone
may not be what stops the title here.

**The abort, runs 14–19: a dead build thread.** The title aborts itself
about 30 s after its frames stop (its deliberate write to 0 at
`+0x23cf683`). In runs 14–19 (and probably 11–13, which had no stack
dump) the frames stopped because a thread building a pipeline died: Wine logged `virtual_setup_exception stack overflow`, the
thread ended without unwinding, the pipeline's `OnceLock` stayed "being
made", and the next title thread that needed it waited forever (run 14's
stuck `OnceLock`, with no thread making it). Translation and builds first
ran on the asking title thread's 1 MiB stack, then on a worker's 2 MiB.
They now get 64 MiB of their own (`b1045749`, `1db4855f`, `7fce94b4`).
With 64 MiB the same shaders still overflowed: `fdd56d97a806bcca` (runs
17, 18), then `a1b20bc47e3c9597` (run 19, with the first denied through
`VICTORY_DENY_PIPELINES`). The recursion was not victory's.

**It was vkd3d-proton's shader converter.** Built from source on Linux, the
dxil-spirv command-line tool reproduces it on the DXIL victory hands
vkd3d-proton (saved with `VICTORY_SHADER_SAVE`, outside the checkout):

| dxil-spirv | Shipped in | `fdd56d97a806bcca` | `a1b20bc47e3c9597` |
|---|---|---|---|
| `114e664e` (2026-11-11) | vkd3d-proton `2576c0b4`, Proton Experimental 2026-11-12 | crash: stack overflow, 64 MiB | crash |
| `b540c8a2` (2026-11-25) | vkd3d-proton master `71cae091` | 207 KB of SPIR-V in 22 ms, `spirv-val` passes | converted |

In `114e664e`, `LoopMergeTracer::trace_from_parent` marks a block visited
only after visiting it, so two loop headers that reach each other recurse
until the stack runs out: 838,609 frames in 64 MiB. Upstream removed that
code with the legacy structurizer (`3bc8276`), and rewrote its other
recursive walks for very large shaders (pull request #371). The title's
shaders are very large: many are 100–300 KB of DXIL and become 0.5–1 MB of
SPIR-V.

Until a Proton release carries a dxil-spirv from 2026-11-25 or later, build
vkd3d-proton master (README) and give the run its two DLLs:
`kc-run.sh --vkd3d DIR` copies them beside the run's own link to
victory.exe, and Wine loads DLLs from the executable's folder first. The
Proton log names the build that loaded (`vkd3d-proton - build: 71cae091…`).

**Runs 20–21 (vkd3d-proton master): both shaders build, and the title stops
at the first big load.** There was no crash and no overflow. About 1m50s
in, the frames stopped and the watchdog fired 30 s later. Two title
threads were recording dispatches whose compute pipelines are made before
the dispatch goes on (`VICTORY_PIPELINE_COMPUTE_WAIT`, on by default).
Hundreds of new pipelines were being made at once, 1–2 s each for compute
and up to 11 s for some mesh pipelines. They were nearly as slow in run 21
as in run 20, so the driver's cache did not help these. Run 20 also
reached the 18 GB cap (VRAM 11.5 of 12 GB).

**Victory's pipeline library never worked under Proton.** Victory asked
for an empty `ID3D12PipelineLibrary` with an empty slice, whose pointer is
never null, and vkd3d-proton refuses a zero length with a pointer
(`E_INVALIDARG`). Windows accepts it. So under Proton no library was made,
no pipeline was kept, and the list of pipelines to start early on the next
launch was never written. The empty library is now asked for with a null
pointer, and each pipeline's name and host time are logged
(`host pipeline made …`, `… loaded from the library`).

**Runs 22–24: the first scenes, then memory.** With the library fixed and
`VICTORY_PIPELINE_COMPUTE_WAIT=0`, a dispatch whose compute pipeline isn't
made yet is dropped after the frame's 500 ms budget, as draws are, instead
of waited for. The picture can be wrong until that pass runs again. The
title then draws through the load into its first 3D scenes: 2,000–6,500
draws a frame at 0.2–0.5 fps while hundreds of pipelines are still being
made. Most of a frame goes to preparing textures (4.2 s of 5.2 s in one
frame of run 23); the GPU's own work is about 0.3 s.

- **Workers and memory.** With 12 pipeline workers, the PC's free memory
  fell by about 6 GB within seconds of the builds starting, and the kernel
  killed run 22 at the 18 GB cap. With 4 (`VICTORY_PIPELINE_WORKERS=4`),
  run 23 held at 15.5–15.9 GB.
- **What the memory is.** In run 23 the process held 7.0 GB of NVIDIA
  driver memory, mapped through `/dev/nvidiactl`: upload and read-back
  heaps, and what no longer fits the card, which was full at 11.9 of 12 GB.
  It also held 5.2 GB of Wine mappings (the title's memory) and 4.7 GB of
  heap.
- **Not the title's needs (corrected).** hyphennn's 23–27 GB is what is
  committed on a 32 GB PC; by their own account about 16 GB of it is in
  memory. The title runs on consoles with about 13.5 GB for everything,
  and on 16 GB PCs. The excess here is victory's (runs 25–26, below).
- **The second run is faster, not instant.** Run 23 loaded run 22's library
  and started 1,621 pipelines early. The 33 pipelines that took over 1 s in
  run 22 (189 s together) took 82 s together in run 23. A known pipeline
  builds faster the second time, but not at cache speed. Offline, the
  driver's cache makes a second build almost free, so why in-game builds
  miss it is still open.

**Run 24 (as run 23, the watchdog fixed): the card overflows into the
PC's memory.** The library and the early list from runs 22–23 had every
pipeline built 1m20s in, before the load. The 3D scenes followed at
2 minutes, at 0.1–0.4 fps with no pipeline left to make. Then the run sat
at the 18 GB cap, the kernel taking back file cache to keep it going,
until it killed the run for memory 9m30s in.

- **The card was full.** Its own memory stood at its 10.2 GB budget, and
  6 GB more of GPU data was in the PC's memory, which vkd3d-proton offers
  the title as a 24 GB "shared" budget.
- **The cap's unreclaimable part grew.** It reached about 17.3 GB: 5.2 GB
  of heap, 6.7 GB of shared memory (the title's memory, Wine's
  `memfd:wine-mapping`) and 5.3 GB the cgroup counts as kernel memory.
- **Only 136 MiB of that "kernel" memory is the kernel's own.** Its stacks,
  page tables and slab come to 136 MiB, so the rest is memory charged by a
  driver: the GPU's data in the PC's memory. It grew about 1.2 GB a minute
  as the title streamed its world in.
- **What fills the cap is victory's, not the title's** (an earlier version
  of this note said the opposite). See runs 25–26.

**Runs 25–26: where the memory goes.** The goal is 12–14 GB for a run, as
on a console. The title's own crash report (run 26, 9m44s in) puts its own
memory at about 8 GB: its tile heap 4.3 of 6.2 GB, its physical memory
2.2 GB, its virtual memory 1.0 GB and its pool 0.6 GB. Wine, vkd3d-proton,
the driver and victory's own allocations (1.3 GB) add 2–3 GB. The rest of
a run's 18 GB is GPU data that victory keeps in the PC's memory:

- **Windows outside buffers are the largest part.** These are copies of
  title memory that the title reads from without a buffer of its own
  (vertices, indices, constants from its allocators). They are cut on a
  4 MiB grid. In run 24, 1,024 of them held 4.7 GB, and only 2.1 GB of that
  had ever been copied: the rest is pages the title never mapped or never
  wrote. NVIDIA's Linux driver allocates and pins every page of a buffer
  in the PC's memory when it is made, read or not, and the run's cgroup
  has no swap. Windows pages out memory that nothing touches.
- **The census (run 25, `VICTORY_INPLACE_PROBE=1`).** Of 790 windows
  (3,856 MB), 439 (2,200 MB) lie over the title's page pool with pages it
  has not mapped; 240 (1,124 MB) over private memory not committed whole,
  or guarded; only 77 (368 MB) over private memory that `VICTORY_INPLACE=1`
  can open in place; 31 (152 MB) over one run of a pool section. The
  probe passes under Proton: vkd3d-proton and the driver open title memory
  as a heap, the GPU reads what the CPU writes there, and a shader's
  write through it is refused (in-place windows only read).
- **In place alone is not enough.** Run 25 had 80 windows (380 MB) in
  place and reached the cap at about the same time as run 24.
- **A finer grid saves memory but churns.** Run 26 cut windows on a
  512 KiB grid, with an ask without a length taking its cell's tail
  (`VICTORY_WINDOW_KB=512 VICTORY_WINDOW_TAILS=1`, hyphennn's switches,
  off by default). Windows held 2.7–3.3 GB, and the run peaked at 16.2
  GB, under the cap for the first time since run 22. But it made windows
  about four times as often, which the 620 driver's bug feeds on (below):
  not recommended.
- **Windows over the page pool in place (runs 27–28).** The pool is a
  committed section under Wine. With `VICTORY_INPLACE_VIEWS=1` (and
  `VICTORY_INPLACE=1`) victory maps the pool pages a window covers, in
  order, into a range of its own (a page of zeros where the title mapped
  none) and opens that range as a heap by its address. Windows refuses
  such a range; vkd3d-proton passes it to the driver, and the probe
  shows the driver takes it: the GPU reads the pages in that order,
  zeros where none, and what the CPU writes afterwards through another
  view. Shaders then read the title's own pages, as on the console, and
  nothing is copied.

  | | run 24 | run 27 | run 28 |
  |---|---|---|---|
  | windows read in place | 0 | 1,461 (6.6 GB) | 1,136 (5.3 GB) |
  | shared buffers made, all run | 6,218 (11.1 GB) | 4,121 (1.4 GB) | 4,274 (1.6 GB) |
  | the driver's memory (cgroup "kernel") | 5–6 GB | 1.8 GB | 0.5–0.6 GB |
  | what could not be reclaimed, at most | 17.8 GB, killed at the cap | 15.0 GB | 13.1 GB |
  | NVIDIA driver errors (`NVRM` lines; failed mapping inserts) | 14; 4 | 0 | 0 |

  That is the 12–14 GB goal, for the first 20 minutes at least.
- **Windows let go, and the slots that showed them.** When the title
  maps a page into a window's range, the window's view of it is out of
  date. Run 27 let a window go at every unmap under it: 2,544 in four
  minutes. Descriptor slots written before then still showed the old
  window: the memory line counted 68,178 looks at a slot that showed
  another copy than the one brought up to date, against 6 and 9 in runs
  24 and 25. Run 28 let a window go only when the title mapped a
  different page (it never mapped the same one back: 0 of 8,721) and made
  again the slots that showed a window let go (870,229 times). Those
  looks still came to 62,903 in 20 minutes. Run 29 let a window go only
  when the changed page lay where something had read through it (2,362
  let go in 8 minutes, 11,786 only marked), and run 30 makes a slot again
  as soon as the pass sees it shows another copy. Those looks still come
  to about 3,000 a minute: about six windows go a second, each shown by
  many slots, and each slot is now made again within a pass of being
  seen. Whether any stays stale longer is not counted yet.
- **The card's budget, and the PC's memory.** Victory stops making
  copies of textures for the card at 90 % of the budget the system gives
  it, but render targets and other copies the host writes are made
  without a test, so the card ends at 98–100 %. Run 30's budget was larger
  (other programs held less of the card), the card reached 99.8 %, and
  vkd3d-proton put 2.4–3.9 GB of GPU data in the PC's memory instead (the
  cgroup's "kernel" figure, 0.5 GB in run 28). That took the run to its
  cap, and the driver's mapping errors came there.

**The title's own watchdog (run 27).** At 0.1–0.2 frames a second the
title's progress watchdog ends a run: a deliberate write to 0 at
`+0x2215c35`, when one of its progress timestamps falls 300 behind. The
hook that skips it is now ported (`VICTORY_NO_WATCHDOG=1`, from
JMAN730/victory `87f33bd6` after #454; it patches only this build, after
checking the bytes), and the `linux-nvidia` profile sets it. kc-run.sh's
own watchdog still ends a run whose output stops.

**Run 28 ended in the title's object pools.** At 16m43s the title stopped
itself on a break (`+0x63966f`). Its crash report shows the same as run
26's: one object pool full (45,000 of 45,000), another nearly (22,371 of
22,500), and the streaming of the opening scene still not finished, with
1,079 requests open. Run 28 had no driver errors at all, so the pools fill
without a GPU stall. Streaming that would take seconds on a console does
not finish here, while the title goes on loading the world around it. Why
is open; it is the next thing to look at after memory.

**Run 32: three speed fixes, and what is left is Wine's remapping.** The
usual frame at 3 minutes fell from about 5 s to 833 ms:

| At 3 minutes | run 31 | run 32 |
|---|---|---|
| usual frame | about 5,000 ms | 833 ms |
| texture refreshes waiting for title memory to hold still | 2,800–3,600 ms | 0 (`VICTORY_TEXTURE_NO_WAIT=1`) |
| victory's own notes of each remap | 1,500–3,400 ms | 117 ms (windows in place indexed by page) |
| views for disk writes | 39 s of the workers' time | 0 (one view of each whole section) |

Later in the run the frames grew to 1.2–2.9 s, nearly all of it remapping:
1,188 calls taking 2.8 s at 6 minutes, of which victory's notes were 1.2 s
again (a window over the pool maps its views while it holds the windows'
lock). A sample over 10 s showed the wineserver at 88 % of one core and
the title's 108 threads waiting, the process at 70 % of one core: every
change of a view goes through that one server thread. The way to 60 frames
a second is fewer such changes, not cheaper ones.

**Moving to trustno1's tree.** trustno1-coder/VIctory-NVIDIA (foxhound base)
runs 26–30 frames a second on Windows against 8–11 for this tree, and
already has its own forms of these fixes: one view of each whole section
for disk writes, texture refreshes that do not wait, and title views kept
in its books until the title's CPU touches them (`VICTORY_SOFT_VIEWS`).
This fork's Linux work goes on there, on the branch `kc/foxhound`.

**Frames got slower.** Runs 27–28 drew at 0.1–0.2 frames a second, with
frames of 5–20 s late in run 28, against about 1 s in run 24 with every
pipeline made. Most of a long frame is the pass over textures shaders pick
by index (13 s of 19.8 s, with 23,403 textures waiting), inside the pass
over the view heap. Open.

**Run 26 froze the desktop: a bug in NVIDIA's 620 driver.** Frames slowed
from about 0.6 s to 3.5–5 s in its last minutes, nearly all of it in
submit calls. At 9m44s the screen froze. The title's process was exiting
(a break in seven of its threads), and its main thread was looping inside
NVIDIA's kernel driver while it closed the device (`nvidia_close` →
`intermapDelDmaMapping` → `btreeUnlink`). It held the driver's global
lock, so the compositor and `nvidia-smi` waited on it (the kernel's
hung-task report). Processes stuck in the driver can't be killed; only a
reboot cleared it.

- **The bug.** In the 620 drivers (here `nvidia-open-dkms` 620.73.04),
  when a new GPU mapping fails to go in because it overlaps one that is
  there ("intermapRegisterDmaMapping: Failed to insert new mapping
  node"), the error path takes a memory object that still has mappings
  out of the driver's table. When that object's last mapping goes, often
  as the process exits, the driver walks a broken table: it loops forever
  holding its lock, or faults. This is
  [NVIDIA/open-gpu-kernel-modules#1540](https://github.com/NVIDIA/open-gpu-kernel-modules/issues/1540),
  with a fix in
  [#1542](https://github.com/NVIDIA/open-gpu-kernel-modules/pull/1542)
  (not merged). In 615.x the same failures did no harm. Proton games of
  all kinds hit it. This PC stays on 620.73.04 for now.
- **The failures.** From 2 minutes in, the driver logged them with
  `gpu_vaspace.c` assertions and "can't update VA space for mapping": 32
  in one minute, 229 in the two minutes before the freeze. Runs 22, 24
  and 25 had a few each, near their memory peaks; run 22 had them
  without in-place windows, so these did not bring them.
- **What makes them likelier here.** Making and freeing many buffers,
  and running short of memory. An issue on hyphennn's fork
  ([#1340](https://github.com/fxceless/victory-fork/issues/1340)) found
  the same, with read-back buffers. Run 26's finer grid and tails made
  14,894 shared buffers (mostly windows) in 10 minutes, against 6,218 in
  run 24's 9m30s: about 14 a second against 3 during play. Its error
  bursts were the worst.
- **The title's crash report** shows two of its object pools full (45,000
  of 45,000 and 22,334 of 22,500 entries); its 13 earlier reports, from
  runs that ended within 3 minutes, had both under half full. Its
  streaming of the opening scene had still not finished. With frames of
  3–5 s and the GPU stalling, this may follow from the stall rather than
  cause it.

**First performance notes (run 24, all pipelines made).** A frame took
about 1 s, and the card was idle for 0.9 s of it; the time goes to the CPU
side (victory's frame log):

| Per frame | ms |
|---|---|
| Remapping title memory (453 calls) | 507 |
| … of which Wine's unmapping of views | 356 (about 0.8 ms a call) |
| … of which Wine's mapping of views | 93 |
| Copying title memory up to the card (144 MB) | 520 |
| Copying back (36 MB) | 200 |
| Refreshing textures (mostly waiting for memory to hold still) | 226 |
| The card's own work | about 170 |

The title remaps a page of its memory for each streamed texture page
(hyphennn's analysis: `CopyPageMappingsBatchX` → `xmem::map_alias`, a view
taken away and one mapped each time). On Windows hyphennn measured about
850 remaps and 140–246 ms in a streaming frame, so 0.2–0.3 ms each. Here
they are about 1.1 ms each, 4–6 times as slow. That makes remapping the
first thing to look at for speed under Proton.

**Runs need no network.** The title's crash handler tried to upload its
report; victory blocked it (`HTTP connection blocked`).

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
