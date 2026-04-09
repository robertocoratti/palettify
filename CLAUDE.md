# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Setup

After cloning, enable the git hooks (runs clippy + tests before every push):

```bash
git config core.hooksPath .githooks
```

## Commands

```bash
# Build the CLI (dev)
cargo build -p palettify-cli

# Build optimized release binary (~5 MB, no runtime deps)
cargo build -p palettify-cli --release

# Run the CLI
cargo run -p palettify-cli -- --help

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p palettify-core
cargo test -p palettify-cli

# Run a specific test file (e.g., color tests)
cargo test -p palettify-core --test color

# Run a specific test by name
cargo test test_name

# Check for compile errors without building
cargo check

# Lint
cargo clippy
```

## Architecture

This is a Cargo workspace with two crates:

```
palettify/
├── Cargo.toml          (workspace)
├── palettes/           (10 built-in YAML palette files, embedded at compile time)
├── core/               (palettify-core — pure library, no I/O)
└── cli/                (palettify-cli — binary, depends on core)
```

**`core/` — `palettify-core`**

Pure Rust library with no I/O. Key modules:

- [core/src/color.rs](core/src/color.rs) — sRGB ↔ OKLab conversion (`rgb_to_oklab`, `srgb_to_linear`, `linear_to_srgb`) and hex parsing (`parse_hex`)
- [core/src/palette.rs](core/src/palette.rs) — `Palette` and `PaletteColor` structs; constructors: `from_hex_list`, `from_yaml`, `from_file`, `load_directory`; nearest-color search in OKLab space
- [core/src/algorithm/](core/src/algorithm/mod.rs) — `Algorithm` enum (`Nearest` | `FloydSteinberg` | `Ordered`) with `from_str` and `run`; each variant in its own submodule (`nearest.rs`, `floyd_steinberg.rs`, `ordered.rs`)
- [core/src/processor.rs](core/src/processor.rs) — thin dispatch: `process_image(img, palette, algorithm) -> RgbImage`
- [core/src/error.rs](core/src/error.rs) — `CoreError` enum

**`cli/` — `palettify-cli`**

Binary crate. Parses args with Clap, embeds palettes via `include_str!`, calls `process_image`, writes output.

- `--palette <name>` — built-in palette by slug
- `--palette-file <path>` — custom YAML palette from disk
- `--output <path>` — output file (default: `<stem>-palettified.<ext>`)
- `--format png|jpg|webp` — output format (default: `png`)
- `--algo nearest|floyd-steinberg|ordered` — algorithm (default: `nearest`)
- `--list-palettes` — print available built-in palettes and exit

**Algorithms** (all operate in OKLab perceptual color space):

| Algorithm | Best for |
|---|---|
| `nearest` | Pixel art, flat-color images — fully parallel (Rayon) |
| `floyd-steinberg` | Photographs, smooth gradients — sequential, row-by-row |
| `ordered` | Retro / Bayer crosshatch look — fully parallel (Rayon) |

**Tests** live in `core/tests/` (one file per module: `color.rs`, `palette.rs`, `processor.rs`, `algorithm.rs`). No inline `#[cfg(test)]` blocks in source files.
