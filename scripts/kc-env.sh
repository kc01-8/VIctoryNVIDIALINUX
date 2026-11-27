# shellcheck shell=bash
# Build environment for cross-compiling victory.exe on Linux.
# Source it, don't run it:   . scripts/kc-env.sh
#
# It holds no secrets and no machine-specific paths. Anything machine-specific
# goes in the untracked ./local.env, which is sourced last if present.

_kc_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

# Self-contained rustup, so the system Rust stays untouched. rust-toolchain.toml
# pins the version, and rustc and the Windows std must come from the same toolchain.
# Install: rustup-init -y --no-modify-path --default-toolchain none --profile minimal
#          (with RUSTUP_HOME/CARGO_HOME set as below)
KC_RUST="${KC_RUST:-$HOME/.local/opt/victory-rust}"
if [ -x "$KC_RUST/cargo/bin/rustup" ]; then
    export RUSTUP_HOME="$KC_RUST/rustup" CARGO_HOME="$KC_RUST/cargo"
    case ":$PATH:" in *":$CARGO_HOME/bin:"*) ;; *) PATH="$CARGO_HOME/bin:$PATH" ;; esac
    export PATH
else
    echo "kc-env: no rustup at $KC_RUST (set KC_RUST); using system cargo" >&2
fi

# LLVM-MinGW (UCRT): newest ~/.local/opt/llvm-mingw-*-ucrt-* unless LLVM_MINGW is set.
if [ -z "${LLVM_MINGW:-}" ]; then
    LLVM_MINGW=$(ls -d "$HOME"/.local/opt/llvm-mingw-*-ucrt-* 2>/dev/null | sort | tail -n 1)
fi
if [ -n "$LLVM_MINGW" ] && [ -x "$LLVM_MINGW/bin/x86_64-w64-mingw32-clang" ]; then
    case ":$PATH:" in *":$LLVM_MINGW/bin:"*) ;; *) PATH="$LLVM_MINGW/bin:$PATH" ;; esac
    export PATH LLVM_MINGW
    export CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER=x86_64-w64-mingw32-clang
else
    echo "kc-env: LLVM-MinGW not found (set LLVM_MINGW or extract it to ~/.local/opt)" >&2
fi

# Short target dir: blake3's ml64 step fails on long paths in MSVC builds.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/vt}"
export XWIN_CACHE_DIR="${XWIN_CACHE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/victory-xwin-x64}"

# Scratch and caches live outside the checkout.
_kc_cache="${XDG_CACHE_HOME:-$HOME/.cache}/victory"
mkdir -p "$_kc_cache/tmp" "$_kc_cache/vkd3d"
export TMPDIR="$_kc_cache/tmp"
export VKD3D_SHADER_CACHE_PATH="${VKD3D_SHADER_CACHE_PATH:-$_kc_cache/vkd3d}"

# DXC Windows DLLs, read from inside the PE, so the path must be a Wine Z: path.
KC_DXC_DIR="${KC_DXC_DIR:-$HOME/.local/opt/dxc}"
export KC_DXC_DIR
export VICTORY_DXC="Z:$(printf '%s' "$KC_DXC_DIR" | tr '/' '\\')"

# The BCPack tables, which you supply yourself (docs/asset-policy.md): kept
# outside the checkout. A Wine Z: path, as for DXC.
KC_BCPACK_TABLES="${KC_BCPACK_TABLES:-$HOME/.local/share/victory/bcpack-tables.bin}"
if [ -f "$KC_BCPACK_TABLES" ]; then
    export VICTORY_BCPACK_TABLES="Z:$(printf '%s' "$KC_BCPACK_TABLES" | tr '/' '\\')"
fi

[ -f "$_kc_root/local.env" ] && . "$_kc_root/local.env"
unset _kc_root _kc_cache
