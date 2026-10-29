#!/usr/bin/env bash
# Checks that a directory holds exactly the installers of a release, nothing more.
# Usage: scripts/release/check-assets.sh 0.1.0 dist/
set -euo pipefail

version="${1:?usage: check-assets.sh X.Y.Z DIR}"
dir="${2:?usage: check-assets.sh X.Y.Z DIR}"

expected=(
  "Keyrust_${version}_amd64.AppImage"
  "Keyrust_${version}_amd64.deb"
  "Keyrust_${version}_universal.dmg"
  "Keyrust_${version}_x64-setup.exe"
  "Keyrust_${version}_x64_en-US.msi"
)

mapfile -t actual < <(find "$dir" -maxdepth 1 -type f -printf '%f\n' | LC_ALL=C sort)
mapfile -t wanted < <(printf '%s\n' "${expected[@]}" | LC_ALL=C sort)

missing=$(LC_ALL=C comm -13 <(printf '%s\n' "${actual[@]}") <(printf '%s\n' "${wanted[@]}"))
extra=$(LC_ALL=C comm -23 <(printf '%s\n' "${actual[@]}") <(printf '%s\n' "${wanted[@]}"))
if [[ -n "$missing" || -n "$extra" ]]; then
  [[ -n "$missing" ]] && printf 'missing: %s\n' "${missing//$'\n'/ }" >&2
  [[ -n "$extra" ]] && printf 'unexpected: %s\n' "${extra//$'\n'/ }" >&2
  exit 1
fi
echo "All ${#expected[@]} installers of $version are present"
