use image::{Rgb, RgbImage};

mod palette;

const THREADS_COUNT: u8 = 8;

pub fn hex_to_rgb(hex: &str) -> Rgb<u8> {
    let hex = if hex.starts_with('#') {
        let mut chars = hex.chars();
        chars.next();
        chars.as_str()
    } else {
        hex
    };
    let rgb = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect::<Result<Vec<u8>, std::num::ParseIntError>>()
        .unwrap_or_default();

    return Rgb::<u8>::from([rgb[0], rgb[1], rgb[2]]);
}

fn main() {
    let start = std::time::Instant::now();
    let mut p = palette::Palette::new("Catppuccin Mocha");

    p.colors.push(hex_to_rgb("#f5e0dc"));
    p.colors.push(hex_to_rgb("#f2cdcd"));
    p.colors.push(hex_to_rgb("#f5c2e7"));
    p.colors.push(hex_to_rgb("#cba6f7"));
    p.colors.push(hex_to_rgb("#f38ba8"));
    p.colors.push(hex_to_rgb("#eba0ac"));
    p.colors.push(hex_to_rgb("#fab387"));
    p.colors.push(hex_to_rgb("#f9e2af"));
    p.colors.push(hex_to_rgb("#a6e3a1"));
    p.colors.push(hex_to_rgb("#94e2d5"));
    p.colors.push(hex_to_rgb("#89dceb"));
    p.colors.push(hex_to_rgb("#74c7ec"));
    p.colors.push(hex_to_rgb("#89b4fa"));
    p.colors.push(hex_to_rgb("#b4befe"));
    p.colors.push(hex_to_rgb("#cdd6f4"));
    p.colors.push(hex_to_rgb("#bac2de"));
    p.colors.push(hex_to_rgb("#a6adc8"));
    p.colors.push(hex_to_rgb("#9399b2"));
    p.colors.push(hex_to_rgb("#7f849c"));
    p.colors.push(hex_to_rgb("#6c7086"));
    p.colors.push(hex_to_rgb("#585b70"));
    p.colors.push(hex_to_rgb("#45475a"));
    p.colors.push(hex_to_rgb("#313244"));
    p.colors.push(hex_to_rgb("#1e1e2e"));
    p.colors.push(hex_to_rgb("#181825"));
    p.colors.push(hex_to_rgb("#11111b"));

    println!("Loading image...");
    let image: RgbImage = image::open("/home/korazza/pictures/wallpapers/demoncore.png")
        .unwrap()
        .to_rgb8();
    println!("Image loaded");

    let image_width = image.width();
    let image_height = image.height();
    let image_size = image_width * image_height;
    let pixels: Vec<&Rgb<u8>> = Vec::from_iter(image.pixels());
    let mut image_result = RgbImage::new(image_width, image_height);

    let chunks = pixels.chunks(((image_size / THREADS_COUNT as u32) as usize).max(1));
    let mut handles = Vec::new();

    println!("Processing...");

    for chunk in chunks {
        let handle = std::thread::spawn(move || {
            let _p = &(p.clone());
            let mut nearest_colors: Vec<Rgb<u8>> = Vec::new();
            for pixel in chunk {
                let nearest_col = palette::nearest_color(_p, pixel);
                nearest_colors.push(nearest_col);
            }
            return nearest_colors;
        });
        handles.push(handle)
    }

    for handle in handles {
        let nearest_colors = handle.join().unwrap();
        for (i, nearest_col) in nearest_colors.iter().enumerate() {
            let x = i as u32 % image_width;
            let y = i as u32 / image_width;
            image_result.get_pixel_mut(x, y).0 = nearest_col.0;
        }
    }

    println!("Saving image...");
    image_result.save("output.png").unwrap();
    println!("Image saved");

    println!("Done in {:?}", start.elapsed());
}
