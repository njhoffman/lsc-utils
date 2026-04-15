#!/usr/bin/env bash
# End-to-end wall-clock comparison between lsc and colorls via hyperfine.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/bench/out"
mkdir -p "$OUT"

if ! command -v hyperfine >/dev/null 2>&1; then
    echo "error: hyperfine not found in PATH." >&2
    echo "install with:  cargo install --locked hyperfine" >&2
    echo "or:            sudo apt install hyperfine / brew install hyperfine" >&2
    exit 2
fi

LSC="${LSC_BIN:-$ROOT/target/release/lsc}"
COLORLS="${COLORLS_BIN:-$HOME/.local/share/gem/ruby/3.3.0/gems/colorls-1.5.0/exe/colorls}"

if [[ ! -x "$LSC" ]]; then
    echo "building release binary..." >&2
    (cd "$ROOT" && cargo build --release --bin lsc)
fi

if [[ ! -x "$COLORLS" ]]; then
    echo "error: colorls not found at $COLORLS (set COLORLS_BIN to override)" >&2
    exit 2
fi

TARGETS=(
    "$ROOT"
    "/usr/bin"
)

MODES=(
    ""          # default grid
    "-l"        # long
    "-a"        # include hidden
    "--tree=2"  # tree, depth 2
)

for target in "${TARGETS[@]}"; do
    [[ -d "$target" ]] || { echo "skipping missing target: $target" >&2; continue; }
    safe="$(echo "$target" | tr '/ ' '__')"
    for mode in "${MODES[@]}"; do
        label="${mode:-default}"
        echo "=== $target  mode=$label ==="
        hyperfine \
            --warmup 3 --runs 20 \
            --export-markdown "$OUT/compare-${safe}-${label// /_}.md" \
            "$LSC $mode -- $target" \
            "$COLORLS $mode -- $target"
    done
done

echo
echo "wrote results to $OUT/"
