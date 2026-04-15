# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`lsc-utils` is a Rust reimplementation of the Ruby gem [colorls](https://github.com/athityakumar/colorls) (1.5.0). The first binary, `lsc`, targets full CLI and output parity with colorls. `TODO.md` tracks high-level intent; this file is the ground truth for working conventions and architecture.

## Status

Feature-complete for v1 parity (0.2.0 milestone). All ten staged steps from `/home/nicholas/.claude/plans/linked-wishing-bumblebee.md` have landed: scaffold, config, theme/icons, CLI + filesystem + grid/one-per-line, long mode, sort matrix, git status (`--gs` via libgit2), tree mode, indicator + hyperlinks, and docs/man page.

Test counts at HEAD: 139 unit tests + 13 integration tests, all green under `just check` (fmt + clippy + test).

## Commands (via `just`)

- `just build` / `just build-release` — cargo build (release enables LTO).
- `just run -- <args>` — run the `lsc` binary via cargo.
- `just test` — all tests (`cargo test --all-features`).
- `just test-one <pattern>` — single test with stdout captured.
- `just fmt` / `just fmt-check` — rustfmt apply / verify.
- `just lint` — `cargo clippy --all-targets --all-features -- -D warnings`.
- `just check` — fmt-check + lint + test (pre-commit gate).
- `just bench` — criterion microbenches.
- `just compare` — `scripts/compare.sh`: hyperfine matrix vs Ruby colorls; writes `bench/out/*.md`. Requires `hyperfine` on PATH.
- `just parity` — `scripts/parity.sh`: captures colorls reference output (SGR-stripped) into `tests/snapshots/colorls/` for comparative development.
- `just snapshot` — `cargo insta review` for golden-output updates.
- `just install` — `cargo install --path . --bin lsc`.
- `just man` — render `man/lsc.1.ronn` via `ronn` (requires ruby-ronn).

## Architecture

Top-level flow: `src/main.rs` → `lsc_utils::run_from_env()` → CLI parse → config load → fs scan → sort/filter → render dispatch by `LayoutMode` → write.

- `src/cli.rs` + `src/options.rs` — clap derive `Args` → canonical `RunOptions` (`LayoutMode`, `SortKey`, `Group`, `TimeStyle`, `IndicatorStyle`, `HyperlinkMode`, `LongOptions`).
- `src/config/` — bundled YAML defaults via `include_str!` deep-merged with user overrides from `~/.config/lsc-utils/{dark,light}_colors.yaml`, `{files,folders,file_aliases,folder_aliases}.yaml`. Layout mirrors colorls so colorls user configs drop in unchanged. `theme.rs` parses CSS named colors (with `csscolorparser`) plus a small alias shim for X11 names colorls uses (`navyblue` → `navy`). `icons.rs` resolves files via extension → alias chain → glyph (extension-only, matches colorls's quirk).
- `src/fs/` — `FileEntry` (lstat + optional symlink target), `scan_directory` with `-a/-A/-d/-f` filters and synthesised `.`/`..`, `sort` comparators (name/size/mtime/ext + dirs-/files-first; proptested for totality and size monotonicity).
- `src/git/` — libgit2 (`git2` with `vendored-libgit2` so builds need no system libgit2). `GitContext::discover` snapshots `Repository::statuses`, aggregates per-file `GitFlags` onto every ancestor under the workdir. `render_status` produces the 4-char `  ✓ ` / per-flag-letter column matching colorls.
- `src/render/` — `cell::CellBuilder` builds pre-styled cells (icon glyph + name with optional `/` indicator and OSC-8 hyperlink wrapping). `grid::render_grid` binary-searches columns to fit `screen_width`, with vertical (column-major) and horizontal (row-major) variants ported from `colorls/lib/colorls/layout.rb`. `long::render` produces the `[inode] mode [hardlinks] [user] [group] size mtime [git] icon name [→ target]` rows joined by 3 spaces. `tree::render` recursively descends with `(dev, inode)` cycle guard and 256-level depth ceiling. `width::display_width` wraps `unicode-width` and bumps Private Use Area codepoints (nerd-font glyphs) to width 2. `hyperlink::wrap` emits OSC-8.
- `src/util/` — `human` size formatting (KiB/MiB/GiB with two decimals + small/medium/large bucket), `mode` permission triplet rendering, `owner`/group lookup with NSS cache and numeric fallback, `time_fmt` via `jiff::strtime` with asctime default + age bucket (hour_old/day_old/no_modifier), `report` counts, tracing init.

## Conventions

- **Debug output**: set `DEBUG=1` to emit tracing debug logs to stderr. `RUST_LOG` overrides the filter; otherwise defaults to `debug`. Never use `println!`/`eprintln!` for diagnostic noise.
- **Error handling**: `thiserror` for typed errors inside modules; `anyhow::Result` at binary/boundary level. No `unwrap()`/`expect()` in production paths — use `?`. Tests freely `unwrap`.
- **Comments**: only when the WHY is non-obvious. No TODO/FIXME in committed code — track work in `TODO.md` or the plan file.
- **Output determinism (tests)**: pin `COLUMNS`, `TZ=UTC`, force `--color=never` and `--without-icons` for assert_cmd integration tests; remove `DEBUG`/`NO_COLOR` from the spawned env.
- **No emojis** in code, comments, commits, or CLI output (the nerd-font icons that `lsc` *renders* are PUA glyphs sourced from the YAML data, which is separate from human-authored text).
- **MSRV**: 1.82 (for `std::iter::repeat_n`).

## Known limitations / follow-ups

- **Locale-aware sort**: Rust stdlib has no `strxfrm`. Currently codepoint order. colorls uses libc strxfrm via the CLocale binding. A small FFI shim under `cfg(unix)` is the cleanest follow-up.
- **Short-mode `--gs`**: git status markers render in long mode only; short modes (grid, one-per-line) accept `--gs` but do not yet display markers. Wiring is straightforward via `CellBuilder` if requested.
- **CI**: no GitHub Actions yet. `just check` is the local gate. Add `cargo deny check` and a stable-Rust matrix once CI is wired.
- **Performance baseline**: `just compare` is wired but baseline numbers aren't checked in. Run it to populate `bench/out/`.

## Reference

- Ruby colorls source (read-only): `~/.local/share/gem/ruby/3.3.0/gems/colorls-1.5.0/`
- Ruby colorls binary (for parity/bench): `~/.local/share/gem/ruby/3.3.0/gems/colorls-1.5.0/exe/colorls`
- Approved implementation plan: `/home/nicholas/.claude/plans/linked-wishing-bumblebee.md`
- Man page source: `man/lsc.1.ronn` — render with `just man`.

## Working directories

Primary: `/home/nicholas/ghq/github.com/njhoffman/lsc-utils`. A sibling checkout at `/home/nicholas/git/lsc-utils` may mirror this one — treat ghq as canonical unless told otherwise.
