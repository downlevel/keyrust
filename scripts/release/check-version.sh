#!/usr/bin/env bash
# Checks that Cargo.toml and the UI package.json carry the same version and, given a tag, that
# the tag is that version. Prints the version on success.
# Usage: scripts/release/check-version.sh [vX.Y.Z]
set -euo pipefail

tag="${1:-}"
root="$(cd "$(dirname "$0")/../.." && pwd)"

# The first `version = "…"` after [workspace.package] in the root manifest.
cargo_version=$(awk '
  /^\[workspace\.package\]/ { section = 1; next }
  /^\[/ { section = 0 }
  section && /^version *=/ { gsub(/.*= *"|".*/, ""); print; exit }
' "$root/Cargo.toml")
ui_version=$(jq -r .version "$root/crates/app/ui/package.json")

if [[ -z "$cargo_version" ]]; then
  echo "error: no version in [workspace.package] of Cargo.toml" >&2
  exit 1
fi
if [[ "$ui_version" != "$cargo_version" ]]; then
  echo "error: crates/app/ui/package.json is $ui_version, Cargo.toml is $cargo_version" >&2
  exit 1
fi
if [[ -n "$tag" && "$tag" != "v$cargo_version" ]]; then
  echo "error: tag $tag does not match version $cargo_version (expected v$cargo_version)" >&2
  exit 1
fi
echo "$cargo_version"
