use palettify_core::algorithm::Algorithm;
use palettify_core::palette::Palette;
use image::{DynamicImage, Rgb, RgbImage};

fn single_pixel(r: u8, g: u8, b: u8) -> DynamicImage {
    let mut img = RgbImage::new(1, 1);
    img.put_pixel(0, 0, Rgb([r, g, b]));
    DynamicImage::ImageRgb8(img)
}

fn bw() -> Palette {
    Palette::from_hex_list("bw", &["#000000", "#ffffff"]).unwrap()
}

// ── FromStr ──────────────────────────────────────────────────────────────────

#[test]
fn from_str_nearest_variants() {
    assert!(matches!("nearest".parse(), Ok(Algorithm::Nearest)));
    assert!(matches!("".parse(), Ok(Algorithm::Nearest)));
    assert!(matches!("  NEAREST  ".parse(), Ok(Algorithm::Nearest)));
}

#[test]
fn from_str_floyd_steinberg_variants() {
    assert!(matches!("floyd-steinberg".parse(), Ok(Algorithm::FloydSteinberg)));
    assert!(matches!("dither".parse::<Algorithm>(), Ok(Algorithm::FloydSteinberg)));
    assert!(matches!("diffusion".parse::<Algorithm>(), Ok(Algorithm::FloydSteinberg)));
    assert!(matches!("FLOYD-STEINBERG".parse::<Algorithm>(), Ok(Algorithm::FloydSteinberg)));
}

#[test]
fn from_str_ordered_variants() {
    assert!(matches!("ordered".parse(), Ok(Algorithm::Ordered)));
    assert!(matches!("bayer".parse::<Algorithm>(), Ok(Algorithm::Ordered)));
    assert!(matches!("ORDERED".parse::<Algorithm>(), Ok(Algorithm::Ordered)));
}

#[test]
fn from_str_unknown_returns_err() {
    assert!("halftone".parse::<Algorithm>().is_err());
    assert!("random".parse::<Algorithm>().is_err());
}

// ── algorithm correctness on trivial inputs ───────────────────────────────────

#[test]
fn nearest_white_maps_to_white() {
    let result = Algorithm::Nearest.run(single_pixel(255, 255, 255), &bw());
    assert_eq!(result.get_pixel(0, 0).0, [255, 255, 255]);
}

#[test]
fn nearest_black_maps_to_black() {
    let result = Algorithm::Nearest.run(single_pixel(0, 0, 0), &bw());
    assert_eq!(result.get_pixel(0, 0).0, [0, 0, 0]);
}

#[test]
fn floyd_steinberg_white_maps_to_white() {
    let result = Algorithm::FloydSteinberg.run(single_pixel(255, 255, 255), &bw());
    assert_eq!(result.get_pixel(0, 0).0, [255, 255, 255]);
}

#[test]
fn ordered_white_maps_to_white() {
    let result = Algorithm::Ordered.run(single_pixel(255, 255, 255), &bw());
    assert_eq!(result.get_pixel(0, 0).0, [255, 255, 255]);
}

#[test]
fn all_algorithms_preserve_dimensions() {
    let palette = Palette::from_hex_list("t", &["#ff0000"]).unwrap();
    let img = DynamicImage::ImageRgb8(RgbImage::new(8, 8));
    for alg in [Algorithm::Nearest, Algorithm::FloydSteinberg, Algorithm::Ordered] {
        let result = alg.run(img.clone(), &palette);
        assert_eq!((result.width(), result.height()), (8, 8));
    }
}
