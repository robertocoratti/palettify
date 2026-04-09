use std::path::PathBuf;

use clap::Parser;
use image::ImageFormat;
use palettify_core::{algorithm::Algorithm, palette::Palette, processor::process_image};

// ── Embedded palettes ────────────────────────────────────────────────────────

const EMBEDDED: &[(&str, &str)] = &[
    ("catppuccin-latte",  include_str!("../../palettes/catppuccin-latte.yaml")),
    ("catppuccin-mocha",  include_str!("../../palettes/catppuccin-mocha.yaml")),
    ("dracula",           include_str!("../../palettes/dracula.yaml")),
    ("gruvbox-dark",      include_str!("../../palettes/gruvbox-dark.yaml")),
    ("gruvbox-light",     include_str!("../../palettes/gruvbox-light.yaml")),
    ("nord",              include_str!("../../palettes/nord.yaml")),
    ("one-dark",          include_str!("../../palettes/one-dark.yaml")),
    ("rose-pine",         include_str!("../../palettes/rose-pine.yaml")),
    ("solarized-dark",    include_str!("../../palettes/solarized-dark.yaml")),
    ("tokyo-night",       include_str!("../../palettes/tokyo-night.yaml")),
];

fn palette_names() -> String {
    EMBEDDED.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
}

// ── CLI definition ───────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "palettify", version, about = "Remap image colors to a palette", arg_required_else_help = true)]
struct Cli {
    /// Source image (PNG, JPEG, WebP, ...)
    input: PathBuf,

    /// Built-in palette name (e.g. nord, dracula)
    #[arg(short, long)]
    palette: Option<String>,

    /// Custom palette file (.yaml)
    #[arg(long)]
    palette_file: Option<PathBuf>,

    /// Output file [default: <input stem>-palettified.<format>]
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format: png | jpg | webp
    #[arg(short, long, default_value = "png")]
    format: String,

    /// Algorithm: nearest | floyd-steinberg | ordered
    #[arg(short, long, default_value = "nearest")]
    algo: String,
}

// ── Entry point ──────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    // Resolve palette.
    let palette = match (&cli.palette, &cli.palette_file) {
        (Some(name), _) => {
            let content = EMBEDDED
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, c)| *c)
                .unwrap_or_else(|| {
                    eprintln!("error: palette '{name}' not found. Available: {}", palette_names());
                    std::process::exit(1);
                });
            Palette::from_yaml(content).unwrap_or_else(|e| {
                eprintln!("error: failed to load palette '{name}': {e}");
                std::process::exit(1);
            })
        }
        (None, Some(path)) => {
            Palette::from_file(path).unwrap_or_else(|e| {
                eprintln!("error: failed to load palette file '{}': {e}", path.display());
                std::process::exit(1);
            })
        }
        (None, None) => {
            eprintln!("error: provide --palette <name> or --palette-file <file>. Available: {}", palette_names());
            std::process::exit(1);
        }
    };

    // Resolve algorithm.
    let algorithm = cli.algo.parse::<Algorithm>().unwrap_or_else(|_| {
        eprintln!("error: unknown algorithm '{}'. Valid values: nearest, floyd-steinberg, ordered", cli.algo);
        std::process::exit(1);
    });

    // Load image.
    let img = image::open(&cli.input).unwrap_or_else(|e| {
        eprintln!("error: could not open '{}': {e}", cli.input.display());
        std::process::exit(1);
    });

    // Process.
    let result = process_image(img, &palette, algorithm);

    // Resolve output format.
    let (image_format, ext) = match cli.format.to_lowercase().as_str() {
        "jpg" | "jpeg" => (ImageFormat::Jpeg, "jpg"),
        "webp"         => (ImageFormat::WebP, "webp"),
        _              => (ImageFormat::Png,  "png"),
    };

    // Resolve output path.
    let output = cli.output.unwrap_or_else(|| {
        let stem = cli.input.file_stem().unwrap_or_default().to_string_lossy();
        cli.input.with_file_name(format!("{stem}-palettified.{ext}"))
    });

    image::DynamicImage::ImageRgb8(result)
        .save_with_format(&output, image_format)
        .unwrap_or_else(|e| {
            eprintln!("error: could not write '{}': {e}", output.display());
            std::process::exit(1);
        });

    println!("saved: {}", output.display());
}
