#!/usr/bin/env bash
# Prints the RUSTFLAGS of a release build: the checkout, Cargo and rustup directories are
# remapped to fixed names, so absolute paths of the build machine do not end up in the binary.
# Usage: RUSTFLAGS="$(scripts/release/rustflags.sh)"
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
rustup_home="${RUSTUP_HOME:-$HOME/.rustup}"

# Git Bash on Windows: rustc sees native paths (C:\…), not /c/….
native() {
  if command -v cygpath >/dev/null 2>&1; then cygpath -w "$1"; else printf '%s' "$1"; fi
}

printf '%s' "--remap-path-prefix=$(native "$root")=/build"
printf ' %s' "--remap-path-prefix=$(native "$cargo_home")=/cargo"
printf ' %s\n' "--remap-path-prefix=$(native "$rustup_home")=/rustup"
