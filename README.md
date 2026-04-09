# palettify

Remap any image's colors to a palette — fast.

Built in Rust with OKLab perceptual color matching and Rayon parallelism.

## Install

```bash
cargo install --path cli
```

Or build manually:

```bash
cargo build -p palettify-cli --release
# binary at: target/release/palettify
```

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

# List available built-in palettes
palettify --list-palettes
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
| `catppuccin-mocha` | 26 |
| `catppuccin-latte` | 25 |
| `dracula` | 11 |
| `gruvbox-dark` | 23 |
| `gruvbox-light` | 22 |
| `nord` | 16 |
| `solarized-dark` | 16 |
| `tokyo-night` | 17 |
| `one-dark` | 15 |
| `rose-pine` | 12 |

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
