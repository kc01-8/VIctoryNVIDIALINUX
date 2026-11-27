#!/usr/bin/env bash
# One reproducible run of a title under Proton: a switch profile, a run
# folder outside the checkout with what was run and what the machine did.
#
#   scripts/kc-run.sh [--profile NAME] [--dry] [--timeout SECONDS] TITLE.exe [-- victory run options]
#
# --profile NAME  switches from profiles/NAME.env (default: default, which is empty)
# --dry           map and bind only (no --execute)
# --timeout S     stop the run after S seconds
#
# The run folder is ~/.local/share/victory/runs/<UTC>-<sha>-<profile>/:
# meta.txt, run.log (victory's output), gpu.csv (nvidia-smi dmon), mem.csv
# (free -m each second), xid.log (NVRM Xid lines from the kernel log, if it
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
while (($#)); do
    case $1 in
        --profile) profile=${2:?--profile needs a name}; shift 2 ;;
        --dry) dry=1; shift ;;
        --timeout) timeout_s=${2:?--timeout needs seconds}; shift 2 ;;
        -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
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
export __GL_SHADER_DISK_CACHE_SIZE=${__GL_SHADER_DISK_CACHE_SIZE:-17179869184}
export __GL_SHADER_DISK_CACHE_SKIP_CLEANUP=${__GL_SHADER_DISK_CACHE_SKIP_CLEANUP:-1}
if [[ -z ${VICTORY_DXC:-} && -n ${KC_DXC_DIR:-} ]]; then
    export VICTORY_DXC=$KC_DXC_DIR
fi
[[ -n ${VICTORY_DXC:-} ]] || echo "warning: VICTORY_DXC is not set; Wine's compiler refuses the helper shaders" >&2

# The profile: KEY=value lines; comments and blank lines ignored.
set -a
# shellcheck source=/dev/null
. "$profile_file"
set +a

sha=$(git -C "$root" rev-parse --short=8 HEAD)
dirty=$(git -C "$root" status --porcelain --untracked-files=no | grep -q . && echo -dirty || true)
stamp=$(date -u +%Y%m%dT%H%M%SZ)
run=$data_home/victory/runs/$stamp-$sha$dirty-$profile
mkdir -p -- "$run"

{
    echo "utc       $stamp"
    echo "commit    $(git -C "$root" rev-parse HEAD)$dirty"
    echo "branch    $(git -C "$root" rev-parse --abbrev-ref HEAD)"
    echo "profile   $profile ($(sha256sum "$profile_file" | cut -c1-16))"
    echo "mode      $([[ $dry == 1 ]] && echo dry || echo execute)$([[ $timeout_s != 0 ]] && echo ", timeout ${timeout_s}s")"
    echo "kernel    $(uname -r)"
    echo "driver    $(nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv,noheader 2>/dev/null || echo unknown)"
    echo "memory    $(free -m | awk '/^Mem:/{print $2 " MiB total, " $7 " MiB available"}'); swap $(free -m | awk '/^Swap:/{print $3 "/" $2 " MiB used"}')"
    echo "proton    ${PROTONPATH:-UMU default} $(ls -d "$HOME"/.local/share/Steam/compatibilitytools.d/UMU-Proton-* 2>/dev/null | sort -V | tail -1 | xargs -r basename)"
    echo "dxc       ${VICTORY_DXC:-unset}"
    echo
    echo "## profile"
    grep -v -E '^\s*(#|$)' "$profile_file" || echo "(empty)"
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

args=("$title")
[[ $dry == 1 ]] || args+=(--execute)
args+=("$@")
echo "run folder: $run"
status=0
if [[ $timeout_s != 0 ]]; then
    timeout --signal=TERM --kill-after=20 "$timeout_s" \
        "$root/scripts/victory-proton.sh" "${args[@]}" 2>&1 | tee "$run/run.log" || status=$?
    # A timeout leaves Wine's processes behind.
    if (( status == 124 || status == 137 )); then
        pkill -f 'victory\.exe' 2>/dev/null || true
        WINEPREFIX=$WINEPREFIX/pfx wineserver -k 2>/dev/null || true
    fi
else
    "$root/scripts/victory-proton.sh" "${args[@]}" 2>&1 | tee "$run/run.log" || status=$?
fi
crashes=$WINEPREFIX/pfx/drive_c/users/steamuser/AppData/Local/victory/data/crashes.log
[[ -f $crashes ]] && cp -- "$crashes" "$run/crashes.log"
echo "exit      $status" >> "$run/meta.txt"
echo "run folder: $run (exit $status)"
exit "$status"
