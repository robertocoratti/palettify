# palettify

Map any image's colors to a palette — fast.

Built in Rust with OKLab perceptual color matching and Rayon parallelism.

## How it works

Three algorithms are available — all operate in **OKLab** perceptual color space:

| Algorithm | Flag | Best for |
|---|---|---|
| Nearest-color (default) | `nearest` | Pixel art, flat-color images — fully parallel (Rayon) |
| Floyd-Steinberg dithering | `floyd-steinberg` | Photographs, smooth gradients — sequential, row-by-row |
| Ordered (Bayer 4×4) dithering | `ordered` | Retro / crosshatch look — fully parallel (Rayon) |

No colorcube precomputation needed — any palette (built-in or custom) works at runtime.
For small palettes (16–30 colors) brute-force over a cache-friendly Vec beats a KD-tree.

## Run

```bash
# Development
cargo run

# Production (optimized binary, ~5 MB, no runtime deps)
cargo build --release
./target/release/palettify
```

Server starts on `http://0.0.0.0:3000` by default. Override with `PORT=8080`.

---

## API

### `GET /health`

```json
{ "status": "ok", "version": "0.2.0" }
```

---

### `GET /api/v1/palettes`

List all built-in palettes.

```json
{
  "palettes": [
    { "name": "catppuccin-mocha", "colors": 26 },
    { "name": "dracula",          "colors": 11 },
    { "name": "gruvbox-dark",     "colors": 23 },
    { "name": "nord",             "colors": 16 },
    ...
  ]
}
```

---

### `POST /api/v1/process`

Process an image with a palette.

**Content-Type:** `multipart/form-data`

| Field          | Type   | Required | Description                                                              |
|----------------|--------|----------|--------------------------------------------------------------------------|
| `image`        | file   | ✅       | Source image (PNG, JPEG, WebP, BMP, …)                                  |
| `palette_name` | string | ✅ or ↓  | Name of a built-in palette (from `/api/v1/palettes`)                    |
| `palette`      | string | ✅ or ↑  | Newline-separated hex colors (`#RRGGBB`), one per line                  |
| `format`       | string | ❌       | Output format: `png` (default) \| `jpg` \| `webp`                       |
| `algorithm`    | string | ❌       | `nearest` (default) \| `floyd-steinberg` \| `ordered`                   |

**Response:** image binary with `Content-Type: image/png` (or jpg/webp)

#### Examples

```bash
# Built-in palette
curl -X POST http://localhost:3000/api/v1/process \
  -F "image=@photo.jpg" \
  -F "palette_name=catppuccin-mocha" \
  -F "format=png" \
  --output result.png

# Custom palette (any number of colors)
curl -X POST http://localhost:3000/api/v1/process \
  -F "image=@photo.jpg" \
  -F $'palette=#ff0000\n#00ff00\n#0000ff\n#ffffff\n#000000' \
  --output result.png

# Floyd-Steinberg dithering for smooth gradients
curl -X POST http://localhost:3000/api/v1/process \
  -F "image=@photo.jpg" \
  -F "palette_name=nord" \
  -F "algorithm=floyd-steinberg" \
  --output result.png

# Ordered (Bayer) dithering for a retro look
curl -X POST http://localhost:3000/api/v1/process \
  -F "image=@photo.jpg" \
  -F "palette_name=gruvbox-dark" \
  -F "algorithm=ordered" \
  --output result.png
```

---

## Built-in palettes

| Slug                | Colors |
|---------------------|--------|
| `catppuccin-mocha`  | 26     |
| `catppuccin-latte`  | 24     |
| `dracula`           | 11     |
| `gruvbox-dark`      | 23     |
| `gruvbox-light`     | 20     |
| `nord`              | 16     |
| `rose-pine`         | 12     |
| `solarized-dark`    | 16     |
| `tokyo-night`       | 17     |
| `one-dark`          | 15     |

## Deploy

Single binary, no runtime dependencies, ~5 MB.

```bash
# Docker
docker build -t palettify .
docker run -p 3000:3000 palettify

# Railway / Fly.io / Render
# Just point to this repo — they'll detect Rust and build automatically
```
