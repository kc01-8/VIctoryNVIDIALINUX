#!/usr/bin/env bash
# Names victory's own frames in a crash report or stack dump.
#
#   scripts/kc-symbolize.sh VICTORY.EXE < victory.log > named.log
#
# Each `victory.exe+0x<rva>` gets " = function (file:line)" appended, from
# the exe's own debug information (the build that ran: release and diag
# builds carry it). Other modules' frames, the title's among them, are left
# as they are. Needs llvm-symbolizer (LLVM-MinGW ships one).
set -euo pipefail
exe=${1:?usage: kc-symbolize.sh VICTORY.EXE < log}
command -v llvm-symbolizer >/dev/null || { echo "llvm-symbolizer not on PATH" >&2; exit 1; }
base=$(llvm-objdump -p "$exe" | awk '/^ImageBase/ && !b {b = $2} END {print b}')
[[ -n $base ]] || { echo "no ImageBase in $exe" >&2; exit 1; }
name=$(basename -- "$exe")
log=$(cat)
# Every distinct rva of this module, symbolized in one llvm-symbolizer call.
mapfile -t rvas < <(grep -oE "${name//./\\.}\+0x[0-9a-fA-F]+" <<<"$log" | sed 's/.*+//' | sort -u)
declare -A sym=()
if ((${#rvas[@]})); then
    addrs=()
    for r in "${rvas[@]}"; do addrs+=("$(printf '0x%x' $((0x$base + r)))"); done
    i=0
    while IFS= read -r fn && IFS= read -r at; do
        sym[${rvas[i]}]="$fn ($(sed 's#.*/crates/#crates/#; s#.*/apps/#apps/#; s#.*/rustc/[0-9a-f]*/#rustc:#' <<<"$at"))"
        i=$((i + 1))
        IFS= read -r _ || true
    done < <(llvm-symbolizer --obj="$exe" --demangle --no-inlines "${addrs[@]}")
fi
while IFS= read -r line; do
    if [[ $line =~ ${name//./\\.}\+(0x[0-9a-fA-F]+) ]] && [[ -n ${sym[${BASH_REMATCH[1]}]:-} ]]; then
        printf '%s = %s\n' "$line" "${sym[${BASH_REMATCH[1]}]}"
    else
        printf '%s\n' "$line"
    fi
done <<<"$log"
