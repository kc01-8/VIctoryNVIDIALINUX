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
| 37 | no profiler | ran 9 minutes until stopped: the stops of runs 35–36 were the profiler's. 1.3–7 frames a second at 600–3,800 draws; the guards' lock: the title's threads waited 57 ms a frame for it in 154 write faults, ours 196 ms |
| 38 | write faults open guarded blocks without the guards' lock (`VICTORY_FAULT_UNLOCKED=1`) | faults no longer wait (257 a frame opened without it); 1.3–15.5 frames a second; the wineserver at 81 % of a core, the whole process at 1.7–1.8 cores |
| 39 | a trace of the wineserver's requests (`WINEDEBUG=+server`) | some 2,000 views mapped and as many unmapped a second, by the page-pool windows in views of our own |
| 40 | (stopped while loading: the profile then overrode the environment; `kc-run.sh` now lets the environment win) | — |
| 41 | page-pool windows as copies (`VICTORY_INPLACE_VIEWS=0`) | the wineserver at 30 %; **8.4–14 frames a second at 1,400–3,700 draws** at 5 minutes, the best yet; but 16.6 GB that could not be reclaimed, 4.4 GB of it the driver's pins of the copies |
| 42 | Wine's compiler's lost root signatures go to DXC; cells let go twice get copies | the depth planes' pass and keeping the card awake work; 880 cells went to copies, and the run reached the 18 GB cap at 3–5 minutes |
| 43 | (no data: the D3D12 device was not made, `E_OUTOFMEMORY`, right after run 42 was killed at the cap) | — |
| 44 | `VICTORY_HOST_LDS_KB=48`; cells' churn counted; a CPU profile with `perf` | 2.2–5.5 frames a second at 1,800–2,700 draws, at most 14.0 GB that could not be reclaimed; 2,533 of 2,534 shaders translated, 2,185 pipelines made, none failed; the meshlet collect still has no 32-lane form; cells let go at least 1, 2, 4, 8, 16 times by 5 minutes: 646, 522, 380, 226, 37; NVIDIA's shader compiler 46 % of the CPU samples (17 % on the title's threads), with the prefix's shader cache empty |
| 45 | NVIDIA's shader cache in a folder of its own, at a size under 4 GiB (`kc-run.sh`'s 16 GiB, 2^34, had left it unwritten) | the cache filled (153 MB); 13.9 GB at most; one 10-second stretch at 13.7 fps over 3,334 draws |
| 46 | the same, a second time; the new numbers panel (copied) | the cache reused: 237 pipelines still being made at 90 s, against 508 at 2 minutes in run 45, none left a minute sooner; 15.6 GB at most |
| 47 | the panel blended over the picture, with a tape of shaders and pipelines | the title stopped in its own crash handler at 1m32s |
| 48 | the same | stopped again, at 1m48s |
| 49 | the panel copied, opaque (`VICTORY_PANEL_BLEND=0`) | the full 6 minutes, 12.4 GB at most; one panel reading of 19.1 fps with the card 78 % busy: blending is off by default since |
| 50 | the title's window borderless (`VICTORY_FULLSCREEN=1`): without it the window was fitted to the work area (2488x1400) and every frame resampled | the full 6 minutes, 12.7 GB at most; the panel's graphs sharp |
| 51 | the panel's letters drawn three times wider and averaged across (lightly hinted) | the full 6 minutes, 11.8 GB at most; the panel's RAM read from the wineserver's books, not ours (fixed after) |
| 52 | trustno1's 64 newer commits brought over (cherry-picks: their history still has the BCPack tables) | 7 minutes, 15.0 GB at most; NVIDIA's compiler 41 % of the CPU's samples (the translator changed, so every pipeline anew); the title's main threads half the rest in one walk of its handle table |
| 53 | `victory play`'s memory stack and texture passes in the profile; the handle table measured | the title stopped at 3 minutes: an unhandled write to read-only title memory, in the table's own code. The table: 15 % full, every key in its home group |
| 54 | write faults under the guards' lock again (`VICTORY_FAULT_UNLOCKED` was in all five runs that ended so: 39, 42, 47, 48, 53); the panel blended again | the full 8 minutes, 12.3 GB at most; 5.6–11.9 fps at 6 minutes; NVIDIA's compiler still 44 % three minutes in, with the cache "warm" |
| 55 | hyphennn's static samplers in one kept order (the driver's cache had answered for nothing: their places differed at every start) | the first start in that order compiled everything; the title waited on a pipeline compiled on its own thread for its last 5 minutes |
| 56 | the same, a second start | **every pipeline made by 90 seconds**, 17 slow; no stall, 8 minutes, 12.1 GB at most; NVIDIA's compiler gone from the profile; 1.1–13.2 fps. What is left: the title's threads' system calls (a quarter of the samples in the kernel) and its own code |
| 57 | a release build (the builds before were `--profile diag` without the `diagnostics` feature, so this compared two builds without diagnostics) | the same speed: 1.9–5.9 fps at 360–3,700 draws, 12.9 GB at most; the panel's painting was 7.3 ms a frame on the presenting thread (since on a thread of its own) |
| 58 | the same, the panel painted on its own thread | the card faulted 2.5 minutes in (NVIDIA Xid 13, "SM Global Exception: Multiple Warp Errors" on a compute channel; device removed). Not seen in any other run |
| 59 | the same again, with the kernel's call chains | the full 7 minutes, 11.9 GB at most. The title's threads: 57.5 % in user space, 25.6 % in the kernel's entries and returns, 6.7 % `ioctl` (Wine's sync objects), 4.0 % `getrusage` (from Wine's `NtDelayExecution`: the title sleeps a great deal), 2.0 % `futex`, 1.2 % `mprotect`; the machine 33 % busy, so frames wait on a chain, not on the processors |
| 60 | the timeline asked for (`VICTORY_TIMELINE`), on a build without the `diagnostics` feature | nothing recorded: `--profile diag` does not turn the feature on |
| 61 | the same with `--features diagnostics` | the title stopped 3 minutes in, as the timeline asked Wine's dbghelp for symbols (a read of -1 inside it): under Wine the timeline names no functions since |
| 62 | the timeline at 3 and 4 minutes | **where frames wait**: of six frames, two on landing jobs (84 and 94 ms), one on a `victory-under` thread outside our spans (1.2 s), one on Wine's address-space calls (277 ms). Over the 2 seconds recorded, the title's threads spent 1.0 s in `VirtualQuery` and 0.6 s in `VirtualProtect`, all of it **inside our guards' write-fault handler**, waiting on Wine's one lock for its address space; the submitting thread 0.3 s in `GetWriteWatch` |
| 63 | a release build, the kernel's call chains | `NtQueryVirtualMemory` 1 sample in 23,394: the queries wait, they do not work. Proton's `ntdll.so` keeps write watches with userfaultfd and `PAGEMAP_SCAN` (no faults, no protection changes) on this kernel |

**Helper shaders.** This tree compiles more of its own shaders at run time
than hyphennn's: the stand-in for the title's ray-traced light
(`ambient.rs`), the card compare, the depth planes' pass, the scatter, the
upload check, the composer. Wine's builtin d3dcompiler_47 refuses most, and
each failure quietly turns its feature off: without the card compare, run
35 compared 3 GB of title buffers on the CPU every frame. All of them now go
through `crate::hlsl::compile_for` (the system's compiler, then DXC at the
6.0 model of the same stage). Wine's compiler also accepts
`[RootSignature(...)]` and leaves it out of the code, and vkd3d-proton then
fails `CreateRootSignature` with E_FAIL (no `RTS0` part): that, not the
driver, was why the depth planes' pass and keeping the card awake failed,
and the scatter, the upload check and the tiles' look were off without a
word. Code without the root signature its source declares now goes to DXC
too (run 42: all of them made).

**Pipelines.** With the pipeline library, every pipeline is made early on a
second start. What is still dropped: the title's ray-tracing passes, which
this tree leaves out without its structure builder (`VICTORY_BVH_BUILDER=1`
turns it on; trustno1 calls it slow and broken on this base), the two named
in `VICTORY_LEAVE_OUT`, and one compute program,
`6b82bc7d22016cb5` (the triangle clip's meshlet collect, groups of 1,024):
it uses the host group's whole 32 KiB of shared memory, so no 32-lane form
fits (`VICTORY_WAVE_WHY=1`: "LDS for the console waves' meetings past the
host's 32 KiB is not supported"). Its 58–65 indirect dispatches a frame
are dropped. `VICTORY_HOST_LDS_KB=48` (in the Linux profile) lets the
translator use Vulkan's 48 KiB on NVIDIA: translated code is not
validated, and vkd3d-proton passes it on. Not yet seen in a run.

Device method `0x218` (`DeserializeComputePipelineStateX`, named by
JMAN730's tree) is not implemented in any tree: it makes compute pipelines
from precompiled native modules for the title's own upscaler, and the
title goes without them.

**Page-pool windows: views or copies.** Under Wine every view mapped or
unmapped is a request to the wineserver, one thread for the whole prefix.
With the windows over the page pool in views of our own the title's
remaps keep it busy (81 % of a core, run 38) and the title's threads wait
behind it; as copies (run 41) the frame rate doubles but NVIDIA's Linux
driver pins the copies, 4.4 GB more. Copies only for cells that churn
(`VICTORY_INPLACE_VIEWS_CHURN`, off by default) did not pay: most cells
churn. The card's memory is full in play (11.4–11.9 GB of 12), so the
copies cannot move there (`VICTORY_WINDOWS_BAR_MB`) either.

**Where a frame goes (run 36).** The card is about 29 % busy; the frame is
the CPU's. The submitting thread works about 214 ms a frame: refreshes 72,
waiting for the card 54, asking the write watch 39. Over all threads Wine's
`VirtualProtect` takes 121 ms a frame for the guards (278 blocks).

**Stops.** Runs 35 and 36 ended at 1.5–2.5 minutes in the title's own
crash handler, both with the sampling profiler on (it suspends threads to
read their stacks); run 34, without it, ran 6.5 minutes until stopped.
On hyphennn's tree the title stopped at 10–17 minutes with an object pool
full while its opening scene was still streaming.
