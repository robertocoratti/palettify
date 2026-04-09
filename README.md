# palettify

[![CI](https://github.com/robertocoratti/palettify/actions/workflows/ci.yml/badge.svg)](https://github.com/robertocoratti/palettify/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/robertocoratti/palettify/branch/main/graph/badge.svg)](https://codecov.io/gh/robertocoratti/palettify)
[![Crates.io](https://img.shields.io/crates/v/palettify.svg)](https://crates.io/crates/palettify)
[![Crates.io Downloads](https://img.shields.io/crates/d/palettify.svg)](https://crates.io/crates/palettify)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg?logo=rust)](https://www.rust-lang.org)

Remap any image's colors to a palette — fast.

Built in Rust with OKLab perceptual color matching and Rayon parallelism.

## Install

```bash
# From crates.io
cargo install palettify

# From source
cargo install --path cli
```

Pre-built binaries for Linux, macOS, and Windows are available on the [releases page](https://github.com/robertocoratti/palettify/releases).

## Usage

```bash
# Built-in palette
palettify photo.jpg --palette nord

# Custom palette file (YAML)
palettify photo.jpg --palette-file my-palette.yaml

# Choose algorithm
palettify photo.jpg --palette dracula --algo floyd-steinberg

# Choose output format and path
palettify photo.jpg --palette catppuccin-mocha --format webp --output out.webp

# List all built-in palettes
palettify --list-palettes

# Inspect a palette (with terminal color swatches)
palettify --show-palette kanagawa
```

## Algorithms

All operate in **OKLab** perceptual color space:

| Flag | Algorithm | Best for |
|---|---|---|
| `nearest` (default) | Nearest-color | Pixel art, flat-color images — fully parallel |
| `floyd-steinberg` | Floyd-Steinberg dithering | Photographs, smooth gradients |
| `ordered` | Ordered / Bayer 4×4 dithering | Retro / crosshatch look — fully parallel |

## Built-in palettes

| Slug | Colors |
|---|---|
| `ayu-dark` | 12 |
| `ayu-light` | 10 |
| `ayu-mirage` | 11 |
| `catppuccin-frappe` | 26 |
| `catppuccin-latte` | 25 |
| `catppuccin-macchiato` | 26 |
| `catppuccin-mocha` | 26 |
| `dracula` | 11 |
| `everforest-dark` | 17 |
| `everforest-light` | 17 |
| `gruvbox-dark` | 23 |
| `gruvbox-light` | 22 |
| `gruvbox-material-dark` | 18 |
| `kanagawa` | 16 |
| `material-ocean` | 12 |
| `material-palenight` | 11 |
| `monokai` | 9 |
| `monokai-pro` | 9 |
| `nightfox` | 18 |
| `nord` | 16 |
| `one-dark` | 15 |
| `oxocarbon` | 16 |
| `poimandres` | 14 |
| `rose-pine` | 12 |
| `rose-pine-dawn` | 15 |
| `rose-pine-moon` | 15 |
| `solarized-dark` | 16 |
| `solarized-light` | 16 |
| `tokyo-night` | 17 |
| `tokyonight-moon` | 17 |
| `tokyonight-storm` | 17 |
| `zenburn` | 13 |

Run `palettify --list-palettes` for a full list with descriptions.

## Custom palette format

Palettes are YAML files. Two color formats are supported:

```yaml
name: my-palette
description: Optional description
colors:
  - "#282a36"              # plain hex
  - background: "#282a36"  # named hex
```

## How it works

OKLab is a perceptual color space where Euclidean distance correlates with human perception of color difference. Matching colors in OKLab produces visually better results than matching in sRGB.

For small palettes (16–30 colors) brute-force over a contiguous `Vec` beats a KD-tree because the entire dataset fits in L1 cache.

All palette files are validated at compile time by `build.rs` (schema structure and per-color hex values). A malformed palette is a build error, not a runtime panic.
