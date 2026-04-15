# lsc-utils

Modern, fast Rust implementation of the [colorls](https://github.com/athityakumar/colorls)
Ruby gem. The first binary, `lsc`, lists directory contents with colors and
nerd-font icons, targeting CLI and output parity with colorls 1.5.0.

## Status

Pre-release (0.2.0). Core listing, long mode, sort matrix, libgit2-backed
git status, tree view, and OSC-8 hyperlinks are all implemented. See
`CLAUDE.md` for the architecture and `man/lsc.1.ronn` for the full flag
reference.

## Install

```sh
just install         # cargo install --path . --bin lsc
```

The binary lands in `~/.cargo/bin/lsc`. Reads YAMLs bundled into the
binary; user overrides go in `~/.config/lsc-utils/` (same layout as
`~/.config/colorls/`).

## Usage

```sh
lsc                       # vertical grid (TTY default)
lsc -1                    # one entry per line
lsc -l                    # long listing
lsc -l --gs               # long with git status column
lsc --tree=2 src          # recursive tree, depth 2
lsc -lS --sd              # long, by size desc, dirs first
lsc --hyperlink=auto      # OSC-8 hyperlinks if the terminal supports them
lsc --without-icons       # disable nerd-font glyphs
```

See `lsc --help` or `man lsc` for the full flag list.

## Configuration

Six YAML files mirroring colorls's layout. Place any subset under
`$XDG_CONFIG_HOME/lsc-utils/` (or `~/.config/lsc-utils/`) and they will
override the bundled defaults entry-by-entry:

- `dark_colors.yaml`, `light_colors.yaml` — color themes (CSS named or
  `#hex`).
- `files.yaml`, `folders.yaml` — file-type / folder-name → glyph.
- `file_aliases.yaml`, `folder_aliases.yaml` — extension/name → canonical
  type.

Existing colorls user configs work unchanged — copy them across.

## Development

`just` runs the workflow:

```sh
just check       # fmt + clippy + tests (pre-commit gate)
just test        # cargo test --all-features
just bench       # criterion microbenches
just compare     # hyperfine comparison vs Ruby colorls (requires hyperfine)
just parity      # capture colorls reference output for diff-driven dev
just install     # cargo install --path . --bin lsc
just man         # render man/lsc.1.ronn (requires ronn-rb)
```

CI is not yet set up; the repo is hosted on GitHub but green-lit locally
via `just check`.

## Architecture

Top-level: `src/main.rs` → `lsc_utils::run_from_env()` → CLI parse →
config load → directory scan → sort/filter → render. Module tree:

- `src/cli.rs` + `src/options.rs` — clap derive surface and canonical
  `RunOptions`.
- `src/config/` — bundled YAML defaults (`include_str!`) deep-merged
  with user overrides; Theme + Icons resolution.
- `src/fs/` — FileEntry (lstat + symlink target), directory scan with
  filtering, sort comparators (proptested for totality + size invariant).
- `src/git/` — libgit2 (vendored) for `--gs`; aggregates per-file flags
  onto every ancestor directory.
- `src/render/` — `cell` (icon + name + color), `grid` (binary-search
  column fitting, ported from colorls's layout.rb), `long`, `tree`,
  `one_per_line`, `hyperlink` (OSC-8), `width` (UAX #11 + nerd-font PUA
  override).
- `src/util/` — human size formatting, mode bits, owner/group cache,
  jiff time-style, report counters, tracing init.

## Performance

Run `just compare` to produce a `hyperfine`-driven comparison vs the
Ruby colorls binary across small/medium/large fixtures and the major
modes (default grid, `-l`, `--tree=2`, `-a`, `-a --gs`). Results land
in `bench/out/`. Targets: ≥10× on plain grid (Ruby startup dominates),
≥3× on `-l`, parity-or-better on `--gs` (libgit2 vs subprocess git).

## Credits

Built on top of the colorls Ruby gem by Athitya Kumar (MIT). The six
YAML data files in `assets/` are copied verbatim from colorls 1.5.0;
see `THIRD_PARTY.md`.

## License

MIT. See `LICENSE`.
