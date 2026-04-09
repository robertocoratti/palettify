use palettify_core::algorithm::Algorithm;
use palettify_core::palette::Palette;
use palettify_core::processor::process_image;
use image::{DynamicImage, Rgb, RgbImage};

fn single_pixel_image(r: u8, g: u8, b: u8) -> DynamicImage {
    let mut img = RgbImage::new(1, 1);
    img.put_pixel(0, 0, Rgb([r, g, b]));
    DynamicImage::ImageRgb8(img)
}

#[test]
fn maps_red_to_nearest_palette_color() {
    let palette = Palette::from_hex_list("test", &["#ff0000", "#0000ff"]).unwrap();
    let img = single_pixel_image(200, 0, 0);
    let result = process_image(img, &palette, Algorithm::Nearest);
    assert_eq!(result.get_pixel(0, 0).0, [255, 0, 0]);
}

#[test]
fn maps_blue_to_nearest_palette_color() {
    let palette = Palette::from_hex_list("test", &["#ff0000", "#0000ff"]).unwrap();
    let img = single_pixel_image(0, 0, 200);
    let result = process_image(img, &palette, Algorithm::Nearest);
    assert_eq!(result.get_pixel(0, 0).0, [0, 0, 255]);
}

#[test]
fn output_dimensions_match_input() {
    let palette = Palette::from_hex_list("test", &["#ffffff"]).unwrap();
    let img = DynamicImage::ImageRgb8(RgbImage::new(100, 80));
    let result = process_image(img, &palette, Algorithm::Nearest);
    assert_eq!(result.width(), 100);
    assert_eq!(result.height(), 80);
}
