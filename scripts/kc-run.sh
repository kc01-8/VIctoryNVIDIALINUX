#!/usr/bin/env bash
# One reproducible run of a title under Proton: a switch profile, a run
# folder outside the checkout with what was run and what the machine did.
#
#   scripts/kc-run.sh [options] TITLE.exe [victory run options] [-- the title's arguments]
#
# What follows the title goes to `victory run` as it is: options of its own
# (`--console series-s`) directly, the title's arguments after a `--`.
#
# --profile NAME  switches from profiles/NAME.env (default: default, which is empty)
# --dry           map and bind only (no --execute)
# --timeout S     stop the run after S seconds
# --mem-max SIZE  run in a systemd user scope with MemoryMax=SIZE and no swap
#                 (for example 18G): past it the kernel kills the run, not the desktop
# --watchdog      kill the run on an NVRM Xid, when its processes use no CPU
#                 for 45 s (a freeze), or when it prints nothing for 300 s
# --vkd3d DIR     use the d3d12.dll and d3d12core.dll in DIR (a vkd3d-proton
#                 build) instead of Proton's: the run gets a bin/ folder with
#                 them beside a hard link (or copy) of victory.exe, and Wine
#                 loads DLLs from the executable's folder first
#
# The run folder is ~/.local/share/victory/runs/<UTC>-<sha>-<profile>/:
# meta.txt, run.log (victory's output), gpu.csv (nvidia-smi dmon), mem.csv
# (free -m each second), mem-cg.csv (with a scope: the run's own memory,
# by kind, every 5 s), xid.log (NVRM Xid lines from the kernel log, if it
# can be read), crashes.log (copied from the prefix after the run). These
# hold what the title printed: they stay outside the checkout and are not
# published (docs/asset-policy.md).
#
# Machine-specific settings (for example PRESSURE_VESSEL_FILESYSTEMS_RO for
# a dump outside $HOME) go in the untracked local.env at the checkout's root.
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/.." && pwd)

profile=default
dry=0
timeout_s=0
mem_max=
watchdog=0
vkd3d=
while (($#)); do
    case $1 in
        --profile) profile=${2:?--profile needs a name}; shift 2 ;;
        --dry) dry=1; shift ;;
        --timeout) timeout_s=${2:?--timeout needs seconds}; shift 2 ;;
        --mem-max) mem_max=${2:?--mem-max needs a size}; shift 2 ;;
        --watchdog) watchdog=1; shift ;;
        --vkd3d) vkd3d=${2:?--vkd3d needs a folder}; shift 2 ;;
        -h|--help) sed -n '2,28p' "$0"; exit 0 ;;
        --) shift; break ;;
        -*) echo "unknown option $1" >&2; exit 2 ;;
        *) break ;;
    esac
done
title=${1:?usage: kc-run.sh [--profile NAME] [--dry] [--timeout S] TITLE.exe [options]}
shift
[[ -f $title ]] || { echo "no such title executable: $title" >&2; exit 1; }
profile_file=$root/profiles/$profile.env
[[ -f $profile_file ]] || { echo "no profile $profile_file" >&2; exit 1; }
if [[ -n $vkd3d ]]; then
    for f in d3d12.dll d3d12core.dll; do
        [[ -f $vkd3d/$f ]] || { echo "no $f in $vkd3d" >&2; exit 1; }
    done
fi

# shellcheck source=/dev/null
[[ -f $root/scripts/kc-env.sh ]] && . "$root/scripts/kc-env.sh"
# shellcheck source=/dev/null
[[ -f $root/local.env ]] && . "$root/local.env"

data_home=${XDG_DATA_HOME:-$HOME/.local/share}
[[ $data_home == /* ]] || data_home=$HOME/.local/share
export WINEPREFIX=${WINEPREFIX:-$data_home/victory/proton}
export GAMEID=${GAMEID:-umu-victory}
export VICTORY_LOG=${VICTORY_LOG:-info}
# A new area builds thousands of pipelines; keep the driver's cache for them.
# Under 4 GiB: run 44's 16 GiB (2^34) left the cache unwritten, as if read
# into 32 bits as 0.
export __GL_SHADER_DISK_CACHE_SIZE=${__GL_SHADER_DISK_CACHE_SIZE:-4294967295}
export __GL_SHADER_DISK_CACHE_SKIP_CLEANUP=${__GL_SHADER_DISK_CACHE_SKIP_CLEANUP:-1}
if [[ -z ${VICTORY_DXC:-} && -n ${KC_DXC_DIR:-} ]]; then
    export VICTORY_DXC=$KC_DXC_DIR
fi
[[ -n ${VICTORY_DXC:-} ]] || echo "warning: VICTORY_DXC is not set; Wine's compiler refuses the helper shaders" >&2

# The profile: KEY=value lines; comments and blank lines ignored. A
# variable already in the environment wins over the profile's line, so one
# switch can be tried against a profile (`VICTORY_X=0 kc-run.sh --profile p`);
# such overrides are listed in meta.txt.
overrides=()
while IFS= read -r line || [[ -n $line ]]; do
    [[ $line =~ ^[[:space:]]*([A-Za-z_][A-Za-z0-9_]*)=(.*)$ ]] || continue
    key=${BASH_REMATCH[1]} value=${BASH_REMATCH[2]}
    if [[ -n ${!key+set} ]]; then
        [[ ${!key} != "$value" ]] && overrides+=("$key=${!key} (profile: $value)")
        continue
    fi
    export "$key=$value"
done < "$profile_file"

sha=$(git -C "$root" rev-parse --short=8 HEAD)
dirty=$(git -C "$root" status --porcelain --untracked-files=no | grep -q . && echo -dirty || true)
stamp=$(date -u +%Y%m%dT%H%M%SZ)
run=$data_home/victory/runs/$stamp-$sha$dirty-$profile
mkdir -p -- "$run"

if [[ -n $vkd3d ]]; then
    # The same default as victory-proton.sh.
    exe=${VICTORY_EXE:-${CARGO_TARGET_DIR:-$root/target}/x86_64-pc-windows-gnullvm/release/victory.exe}
    [[ -f $exe ]] || { echo "no victory.exe at $exe (set VICTORY_EXE)" >&2; exit 1; }
    mkdir -p -- "$run/bin"
    ln -f -- "$exe" "$run/bin/victory.exe" 2>/dev/null || cp -- "$exe" "$run/bin/victory.exe"
    cp -- "$vkd3d/d3d12.dll" "$vkd3d/d3d12core.dll" "$run/bin/"
    export VICTORY_EXE=$run/bin/victory.exe
fi

{
    echo "utc       $stamp"
    echo "commit    $(git -C "$root" rev-parse HEAD)$dirty"
    echo "branch    $(git -C "$root" rev-parse --abbrev-ref HEAD)"
    echo "profile   $profile ($(sha256sum "$profile_file" | cut -c1-16))"
    echo "mode      $([[ $dry == 1 ]] && echo dry || echo execute)$([[ $timeout_s != 0 ]] && echo ", timeout ${timeout_s}s")$([[ -n $mem_max ]] && echo ", MemoryMax=$mem_max, no swap")$([[ $watchdog == 1 ]] && echo ", watchdog")"
    echo "kernel    $(uname -r)"
    echo "driver    $(nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv,noheader 2>/dev/null || echo unknown)"
    echo "memory    $(free -m | awk '/^Mem:/{print $2 " MiB total, " $7 " MiB available"}'); swap $(free -m | awk '/^Swap:/{print $3 "/" $2 " MiB used"}')"
    echo "proton    ${PROTONPATH:-UMU default} $(ls -d "$HOME"/.local/share/Steam/compatibilitytools.d/UMU-Proton-* 2>/dev/null | sort -V | tail -1 | xargs -r basename)"
    echo "dxc       ${VICTORY_DXC:-unset}"
    echo "exe       ${VICTORY_EXE:-default} $( [[ -n ${VICTORY_EXE:-} ]] && sha256sum "$VICTORY_EXE" | cut -c1-16)"
    if [[ -n $vkd3d ]]; then
        echo "vkd3d     $vkd3d: $(head -c 200 "$vkd3d/version" 2>/dev/null || echo 'no version file') ($(sha256sum "$vkd3d/d3d12core.dll" | cut -c1-16))"
    else
        echo "vkd3d     Proton's"
    fi
    echo
    echo "## profile"
    grep -v -E '^\s*(#|$)' "$profile_file" || echo "(empty)"
    if ((${#overrides[@]})); then
        echo
        echo "## overridden from the environment"
        printf '%s\n' "${overrides[@]}"
    fi
    echo
    echo "## git diff --stat"
    git -C "$root" diff --stat HEAD
} > "$run/meta.txt"

pids=()
nvidia-smi dmon -s pucvmet -d 1 > "$run/gpu.csv" 2>&1 & pids+=($!)
( while :; do printf '%s ' "$(date -u +%T)"; free -m | awk '/^Mem:/{printf "%s %s ", $3, $7} /^Swap:/{print $3}'; sleep 1; done ) > "$run/mem.csv" & pids+=($!)
if journalctl -k -n 0 >/dev/null 2>&1; then
    journalctl -kf -n 0 -g 'NVRM: Xid' -o short-iso > "$run/xid.log" 2>&1 & pids+=($!)
else
    echo "kernel log not readable by this user; no Xid capture" > "$run/xid.log"
fi
stop() {
    kill "${pids[@]}" 2>/dev/null || true
    wait "${pids[@]}" 2>/dev/null || true
}
trap stop EXIT

# The run's own cgroup, so it can be capped, measured and killed as a whole.
unit=kc-run-$stamp
runner=()
if [[ -n $mem_max || $watchdog == 1 ]]; then
    runner=(systemd-run --user --scope --quiet --unit="$unit" --collect)
    [[ -n $mem_max ]] && runner+=(-p "MemoryMax=$mem_max" -p MemorySwapMax=0)
    runner+=(--)
fi
cgroup() {
    local p
    p=$(systemctl --user show -P ControlGroup "$unit.scope" 2>/dev/null) || return 1
    [[ -n $p ]] && printf '/sys/fs/cgroup%s' "$p"
}
kill_run() {
    echo "watchdog  $1 at $(date -u +%T)" >> "$run/meta.txt"
    echo "kc-run: watchdog: $1; killing the run" >&2
    systemctl --user kill --signal=KILL "$unit.scope" 2>/dev/null || true
}
# Every 5 s: the cgroup's peak memory and OOM kills to mem-peak.txt, and
# with --watchdog the three kill rules.
monitor() {
    local cg cpu last_cpu=-1 still=0 size last_size=-1 quiet=0
    while :; do
        sleep 5
        cg=$(cgroup) || continue
        [[ -d $cg ]] || continue
        printf 'peak %s MiB, oom_kill %s\n' \
            "$(( $(cat "$cg/memory.peak" 2>/dev/null || echo 0) >> 20 ))" \
            "$(awk '/^oom_kill /{print $2}' "$cg/memory.events" 2>/dev/null)" > "$run/mem-peak.txt"
        # What the cgroup's memory is: anonymous, file cache, shared, kernel (MiB).
        [[ -s $run/mem-cg.csv ]] || echo "time current anon file shmem kernel" > "$run/mem-cg.csv"
        awk -v t="$(date -u +%T)" -v cur="$(cat "$cg/memory.current" 2>/dev/null || echo 0)" \
            '$1 == "anon" || $1 == "file" || $1 == "shmem" || $1 == "kernel" {v[$1] = int($2 / 1048576)}
             END {printf "%s %d %d %d %d %d\n", t, cur / 1048576, v["anon"], v["file"], v["shmem"], v["kernel"]}' \
            "$cg/memory.stat" >> "$run/mem-cg.csv" 2>/dev/null || true
        [[ $watchdog == 1 ]] || continue
        if grep -q 'Xid' "$run/xid.log" 2>/dev/null; then
            kill_run "NVRM Xid: $(grep -m1 Xid "$run/xid.log")"; return
        fi
        cpu=$(awk '/^usage_usec /{print $2}' "$cg/cpu.stat" 2>/dev/null || echo 0)
        if (( cpu == last_cpu )); then still=$((still + 5)); else still=0; fi
        last_cpu=$cpu
        if (( still >= 45 )); then kill_run "no CPU time used for ${still}s (frozen)"; return; fi
        # Output is run.log and victory.log together: under Proton
        # Experimental victory's log goes only to victory.log (no console).
        size=$(( $(stat -c %s "$run/run.log" 2>/dev/null || echo 0) + $(stat -c %s "$run/victory.log" 2>/dev/null || echo 0) ))
        if (( size == last_size )); then quiet=$((quiet + 5)); else quiet=0; fi
        last_size=$size
        if (( quiet >= 300 )); then kill_run "no output for ${quiet}s"; return; fi
    done
}
if ((${#runner[@]})); then
    monitor & pids+=($!)
fi

# Victory's own log into the run folder: Proton Experimental launches the
# title without a console, so its stderr does not reach run.log.
export VICTORY_LOG_FILE="Z:${run//\//\\}\\victory.log"
# crashes.log is kept per title under the prefix's data folder and grows
# across runs: note its size now, copy what this run adds.
data_dir=$WINEPREFIX/pfx/drive_c/users/steamuser/AppData/Local/victory/data
declare -A crash_size=()
for f in "$data_dir"/*/crashes.log; do
    [[ -f $f ]] && crash_size[$f]=$(stat -c %s "$f")
done

args=("$title")
[[ $dry == 1 ]] || args+=(--execute)
args+=("$@")
echo "run folder: $run"
status=0
if [[ $timeout_s != 0 ]]; then
    timeout --signal=TERM --kill-after=20 "$timeout_s" \
        "${runner[@]}" "$root/scripts/victory-proton.sh" "${args[@]}" 2>&1 | tee "$run/run.log" || status=$?
    # A timeout leaves Wine's processes behind.
    if (( status == 124 || status == 137 )); then
        if ((${#runner[@]})); then
            systemctl --user kill --signal=KILL "$unit.scope" 2>/dev/null || true
        else
            pkill -f 'victory\.exe' 2>/dev/null || true
        fi
    fi
else
    "${runner[@]}" "$root/scripts/victory-proton.sh" "${args[@]}" 2>&1 | tee "$run/run.log" || status=$?
fi
for f in "$data_dir"/*/crashes.log; do
    [[ -f $f ]] || continue
    from=${crash_size[$f]:-0}
    if (( $(stat -c %s "$f") > from )); then
        tail -c +$((from + 1)) -- "$f" >> "$run/crashes.log"
    fi
done
echo "exit      $status" >> "$run/meta.txt"
[[ -f $run/mem-peak.txt ]] && echo "memory    $(cat "$run/mem-peak.txt") (the run's cgroup)" >> "$run/meta.txt"
echo "run folder: $run (exit $status)"
exit "$status"
