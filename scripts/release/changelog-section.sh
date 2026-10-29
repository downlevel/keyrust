#!/usr/bin/env bash
# Prints the CHANGELOG.md section of one version, without its heading.
# Usage: scripts/release/changelog-section.sh 0.1.0
set -euo pipefail

version="${1:?usage: changelog-section.sh X.Y.Z}"
root="$(cd "$(dirname "$0")/../.." && pwd)"

section=$(awk -v heading="## [$version]" '
  index($0, heading) == 1 { found = 1; next }
  found && /^## \[/ { exit }
  # The link definitions at the end of the file close the last section.
  found && /^\[[^]]+\]: / { exit }
  found { print }
' "$root/CHANGELOG.md")

# Drop leading and trailing blank lines.
section=$(printf '%s\n' "$section" | sed -e '/./,$!d' | sed -e ':a' -e '/^\n*$/{$d;N;ba' -e '}')
if [[ -z "$section" ]]; then
  echo "error: CHANGELOG.md has no section for $version" >&2
  exit 1
fi
printf '%s\n' "$section"
