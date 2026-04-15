# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`lsc-utils` is a Rust reimplementation of the Ruby gem [colorls](https://github.com/athityakumar/colorls) (1.5.0). The first binary, `lsc`, targets full CLI and output parity with colorls. `TODO.md` tracks high-level intent; this file is the ground truth for working conventions as implementation lands.

## Status

Scaffolded (step 1 of 10). `Cargo.toml`, `src/main.rs`, `src/lib.rs`, `justfile`, lint/format configs, license, and comparison scripts are in place. The binary is a no-op stub until CLI parsing lands (step 4). Full implementation plan: `/home/nicholas/.claude/plans/linked-wishing-bumblebee.md`.

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

Top-level flow: `src/main.rs` → `lsc_utils::run_from_env()` → CLI parse → config load → fs scan → sort/filter → render. Module tree (lands across steps 2–9):

- `src/cli.rs` + `src/options.rs` — clap derive Args → canonical `RunOptions` (LayoutMode, SortKey, TimeStyle, Group).
- `src/config/` — bundled YAML defaults (via `rust-embed`) deep-merged with user overrides from `~/.config/lsc-utils/{dark_colors,light_colors,files,file_aliases,folders,folder_aliases,config}.yaml`. Layout mirrors colorls so colorls user configs drop in unchanged.
- `src/fs/` — `FileEntry` (lstat + optional target_meta), hidden filters (`-a`/`-A`/`-d`/`-f`), sort comparators (name/size/mtime/ext + dirs/files-first), indicator suffixes.
- `src/git/` — libgit2 (via `git2` with vendored-libgit2) discovers the enclosing repo once per invocation, maps paths → `GitStatus` bitflags. Scoped to listed dir via pathspec to keep large repos fast.
- `src/render/` — `trait LayoutEngine` with impls for VerticalGrid, HorizontalGrid, Long, OnePerLine, Tree. Grid fits columns via binary search mirroring colorls's algorithm. `render/width.rs` wraps `unicode-width` with a curated override table for nerd-font PUA glyphs (which render 2 wide despite UAX#11 reporting 1) and ZWJ collapse. `render/hyperlink.rs` emits OSC-8 under `--hyperlink`.
- `src/util/` — human-readable sizes, `--time-style` via `jiff::strtime`, `--report` summary, tracing init.
- `assets/*.yaml` — the six YAML tables copied verbatim from colorls under MIT (see `THIRD_PARTY.md`).

## Conventions

- **Debug output**: set `DEBUG=1` to emit tracing debug logs to stderr. The shim lives in `src/lib.rs::init_debug_tracing`; honors `RUST_LOG` if already set. Never use `println!`/`eprintln!` for diagnostic noise.
- **Error handling**: `thiserror` for typed errors inside modules; `anyhow::Result` at binary/boundary level. No `unwrap()`/`expect()` in production paths — use `?`.
- **Comments**: only when the WHY is non-obvious (see repo root policy). No TODO/FIXME in committed code — track work in `TODO.md` or the plan file.
- **Output determinism (tests)**: pin `COLUMNS`, `TZ=UTC`, force `--color=always`, stub uid/gid via test harness. `insta` filters mask mtimes/inodes.
- **No emojis** in code, comments, commits, or CLI output (emojis-as-icons are nerd-font glyphs, which is separate — they ship through `assets/`).

## Reference

- Ruby colorls source (read-only): `~/.local/share/gem/ruby/3.3.0/gems/colorls-1.5.0/`
- Ruby colorls binary (for parity/bench): `~/.local/share/gem/ruby/3.3.0/gems/colorls-1.5.0/exe/colorls`
- Approved implementation plan: `/home/nicholas/.claude/plans/linked-wishing-bumblebee.md`

## Working directories

Primary: `/home/nicholas/ghq/github.com/njhoffman/lsc-utils`. A sibling checkout at `/home/nicholas/git/lsc-utils` may mirror this one — treat ghq as canonical unless told otherwise.
