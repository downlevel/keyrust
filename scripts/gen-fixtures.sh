#!/usr/bin/env bash
# Regenerates the KeePassXC-created test fixtures from their XML sources.
#
# The password below is test-only and intentionally public: fixtures never
# contain real credentials. It is passed on stdin, never on the command line.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES="$ROOT/tests/fixtures"
CLI="${KEEPASSXC_CLI:-keepassxc-cli}"
PASSWORD="keyrust-test-password"

command -v "$CLI" >/dev/null || { echo "error: $CLI not found (install KeePassXC)" >&2; exit 1; }

out="$FIXTURES/keepassxc-rich.kdbx"
rm -f "$out"
# `import -p` asks for the new password twice; `-t 100` keeps the KDF fast for tests.
printf '%s\n%s\n' "$PASSWORD" "$PASSWORD" |
    "$CLI" import -q -p -t 100 "$FIXTURES/src/rich.xml" "$out" 2>/dev/null

echo "Generated $out with $("$CLI" --version | head -n1 | sed 's/^/KeePassXC /')"
printf '%s\n' "$PASSWORD" | "$CLI" db-info -q "$out"

# KDBX 3.1: what `db-create` produces in KeePassXC 2.7, with one entry.
out31="$FIXTURES/keepassxc-kdbx31.kdbx"
rm -f "$out31"
printf '%s\n%s\n' "$PASSWORD" "$PASSWORD" | "$CLI" db-create -q -p -t 100 "$out31" 2>/dev/null
printf '%s\n%s\n' "$PASSWORD" "legacy-entry-password" |
    "$CLI" add -q -u alice --url https://legacy.example -p "$out31" Legacy 2>/dev/null
echo "Generated $out31"
printf '%s\n' "$PASSWORD" | "$CLI" db-info -q "$out31"
