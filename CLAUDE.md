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
cargo build -p palettify

# Build optimized release binary (~5 MB, no runtime deps)
cargo build -p palettify --release

# Run the CLI
cargo run -p palettify -- --help

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p palettify-core
cargo test -p palettify

# Run a specific test file (e.g., color tests)
cargo test -p palettify-core --test color

# Run a specific test by name
cargo test test_name

# Check for compile errors without building
cargo check

# Lint
cargo clippy

# Coverage (requires cargo-llvm-cov: cargo install cargo-llvm-cov)
cargo llvm-cov
cargo llvm-cov --html   # generates target/llvm-cov/html/index.html
```

## Architecture

This is a Cargo workspace with two crates:

```
palettify/
├── Cargo.toml              (workspace)
├── palettes/               (32 built-in YAML palette files — source of truth)
├── core/                   (palettify-core — pure library, no I/O)
│   └── tests/              (integration tests: color.rs, palette.rs, processor.rs, algorithm.rs)
└── cli/                    (palettify — binary, depends on core)
    ├── build.rs            (reads palettes/, validates schema, generates palettes_generated.rs)
    └── src/
        ├── main.rs
        └── palettes_generated.rs   [gitignored — recreated on every cargo build]
```

**`palettes/`**

32 YAML files at the workspace root. Each file is the canonical source for a built-in palette. `build.rs` reads this directory at compile time, validates every file (schema + per-color hex), and embeds the content into the binary. Adding a new palette = drop a `.yaml` file here, rebuild.

**`cli/build.rs`**

Runs before compilation. Validates all `palettes/*.yaml` (required fields, non-empty colors list, valid hex per color), then generates `cli/src/palettes_generated.rs` with a `const EMBEDDED: &[(&str, &str)]` array of `(slug, yaml_content)` pairs. Writing to `src/` (not `$OUT_DIR`) makes the file visible to rust-analyzer without any special configuration.

**`core/` — `palettify-core`**

Pure Rust library with no I/O. Key modules:

- [core/src/color.rs](core/src/color.rs) — sRGB ↔ OKLab conversion (`rgb_to_oklab`, `srgb_to_linear`, `linear_to_srgb`) and hex parsing (`parse_hex`)
- [core/src/palette.rs](core/src/palette.rs) — `Palette` and `PaletteColor` structs; constructors: `from_hex_list`, `from_yaml`, `from_file`, `load_directory`; nearest-color search in OKLab space
- [core/src/algorithm/](core/src/algorithm/mod.rs) — `Algorithm` enum (`Nearest` | `FloydSteinberg` | `Ordered`) implementing `FromStr`; each variant in its own submodule (`nearest.rs`, `floyd_steinberg.rs`, `ordered.rs`)
- [core/src/processor.rs](core/src/processor.rs) — thin dispatch: `process_image(img, palette, algorithm) -> RgbImage`
- [core/src/error.rs](core/src/error.rs) — `CoreError` enum

**`cli/` — `palettify`**

Binary crate. Parses args with Clap, includes the generated palette array, calls `process_image`, writes output.

- `-l, --list-palettes` — print all built-in palettes (slug, color count, description) and exit
- `--show-palette <name>` — print all colors of a palette with ANSI true-color swatches and exit
- `-p, --palette <name>` — built-in palette by slug
- `--palette-file <path>` — custom YAML palette from disk
- `-o, --output <path>` — output file (default: `<stem>-palettified.<ext>`)
- `-f, --format png|jpg|webp` — output format (default: `png`)
- `-a, --algo nearest|floyd-steinberg|ordered` — algorithm (default: `nearest`)

**Algorithms** (all operate in OKLab perceptual color space):

| Algorithm | Best for |
|---|---|
| `nearest` | Pixel art, flat-color images — fully parallel (Rayon) |
| `floyd-steinberg` | Photographs, smooth gradients — sequential, row-by-row |
| `ordered` | Retro / Bayer crosshatch look — fully parallel (Rayon) |

**Tests** live in `core/tests/` (one file per module: `color.rs`, `palette.rs`, `processor.rs`, `algorithm.rs`). No inline `#[cfg(test)]` blocks in source files.
