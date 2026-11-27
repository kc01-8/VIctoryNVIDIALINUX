# Linux status of `kc/foxhound`

trustno1-coder/VIctory-NVIDIA (foxhound base) under Proton on one PC:
RTX 3080 Ti (12 GB, driver 620.73.04), 32 GB RAM, Proton Experimental
2026-11-12 with vkd3d-proton built from master (`71cae091`), `scripts/kc-run.sh
--profile linux-foxhound --mem-max 18G --watchdog`, diagnostics build, Enter
pressed once 18 s in. The runs before these, on hyphennn's tree, are in
`docs/kc-linux-status.md` of `kc/nvidia` (runs 3–32).

| Run | What changed | What happened |
|---|---|---|
| 33 | this tree as it came, with the DXC fallback for the indirect clamp, the empty pipeline library and the watchdog hook | 1.6–1.9 frames a second at about 3,100 draws, against 1 at about 870 on `kc/nvidia`; 870–886 pipelines still being made (a cold start); killed at the 18 GB cap after 2m20s, 5.8 GB of it the driver's |
| 34 | windows of title memory in place, those over the page pool through views of our own (`VICTORY_INPLACE=1 VICTORY_INPLACE_VIEWS=1`) | the driver's memory 0.2 GB, at most 12.9 GB that could not be reclaimed; 1.4–2.2 frames a second; stopped by hand at 6.5 minutes |
| 35 | the sampling profiler (`VICTORY_PROFILE`) | the title stopped itself at 2m29s, the profile not written; it lost, to Wine's compiler, the stand-in for the indirect light, the card compare, the depth planes' pass and keeping the card awake (below) |
| 36 | every helper shader compiled at run time falls back to DXC; refused write-watch askings not asked again | 3–5.6 frames a second at 1,600–2,400 draws (179 ms a frame at best); no CPU snapshots for shader writes (187 ms a frame before); the title stopped itself at 1m30s, again with the profiler on |

**Helper shaders.** This tree compiles more of its own shaders at run time
than hyphennn's: the stand-in for the title's ray-traced light
(`ambient.rs`), the card compare, the depth planes' pass, the scatter, the
upload check, the composer. Wine's builtin d3dcompiler_47 refuses most, and
each failure quietly turns its feature off: without the card compare, run
35 compared 3 GB of title buffers on the CPU every frame. All of them now go
through `crate::hlsl::compile_for` (the system's compiler, then DXC at the
6.0 model of the same stage). Still failing: the depth planes' pipeline
(E_FAIL when it is made, not when compiled), and keeping the card awake
(its own command queue: the driver refuses more channel groups, `GspRmAlloc
failed … hClass=0xa06c` in the kernel log).

**Pipelines.** With the pipeline library, every pipeline is made early on a
second start. What is still dropped: the title's ray-tracing passes, which
this tree leaves out without its structure builder (`VICTORY_BVH_BUILDER=1`
turns it on; trustno1 calls it slow and broken on this base), the two named
in `VICTORY_LEAVE_OUT`, and one compute program,
`6b82bc7d22016cb5` (the triangle clip's meshlet collect, groups of 1,024):
it uses the host group's whole 32 KiB of shared memory, so no 32-lane form
fits (`VICTORY_WAVE_WHY=1`: "LDS for the console waves' meetings past the
host's 32 KiB is not supported"). Its 58–65 indirect dispatches a frame
are dropped.

**Where a frame goes (run 36).** The card is about 29 % busy; the frame is
the CPU's. The submitting thread works about 214 ms a frame: refreshes 72,
waiting for the card 54, asking the write watch 39. Over all threads Wine's
`VirtualProtect` takes 121 ms a frame for the guards (278 blocks).

**Stops.** Runs 35 and 36 ended at 1.5–2.5 minutes in the title's own
crash handler, both with the sampling profiler on (it suspends threads to
read their stacks); run 34, without it, ran 6.5 minutes until stopped.
On hyphennn's tree the title stopped at 10–17 minutes with an object pool
full while its opening scene was still streaming.
