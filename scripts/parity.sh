#!/usr/bin/env bash
# Capture reference output from the Ruby colorls for parity comparison.
# Strips ANSI SGR so diffs focus on layout/content, not exact color codes.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/tests/snapshots/colorls"
mkdir -p "$OUT"

COLORLS="${COLORLS_BIN:-$HOME/.local/share/gem/ruby/3.3.0/gems/colorls-1.5.0/exe/colorls}"
if [[ ! -x "$COLORLS" ]]; then
    echo "error: colorls not found at $COLORLS" >&2
    exit 2
fi

strip_sgr() { sed -E $'s/\x1b\\[[0-9;]*[a-zA-Z]//g'; }

FIXTURE_ROOT="${FIXTURE_ROOT:-$ROOT/tests/fixtures}"
if [[ ! -d "$FIXTURE_ROOT" ]]; then
    echo "note: no fixtures at $FIXTURE_ROOT yet; falling back to /usr/bin sample" >&2
    FIXTURE_ROOT="/usr/bin"
fi

export COLUMNS=120
export TERM=xterm-256color

for mode_label in "default:" "long:-l" "all:-a" "tree2:--tree=2" "one:-1"; do
    name="${mode_label%%:*}"
    flags="${mode_label#*:}"
    # shellcheck disable=SC2086
    "$COLORLS" --color=always $flags -- "$FIXTURE_ROOT" | strip_sgr > "$OUT/${name}.txt"
    echo "wrote $OUT/${name}.txt"
done
