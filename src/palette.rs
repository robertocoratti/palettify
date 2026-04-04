use std::collections::HashMap;

use image::Rgb;

fn colors_distance(color_1: &Rgb<u8>, color_2: &Rgb<u8>) -> f32 {
    let r_diff = (color_1[0] as f32) - (color_2[0] as f32);
    let g_diff = (color_1[1] as f32) - (color_2[1] as f32);
    let b_diff = (color_1[2] as f32) - (color_2[2] as f32);
    return (r_diff.powf(2.0) + g_diff.powf(2.0) + b_diff.powf(2.0)).sqrt();
}

#[derive(Clone)]
pub struct Palette {
    pub name: &'static str,
    pub colors: Vec<Rgb<u8>>,
    nearest_colors_cache: HashMap<Rgb<u8>, usize>,
}

impl Palette {
    pub fn new(name: &'static str) -> Palette {
        return Palette {
            name,
            colors: Vec::new(),
            nearest_colors_cache: HashMap::new(),
        };
    }
}

pub fn nearest_color(palette: &Palette, color: &Rgb<u8>) -> Rgb<u8> {
    if palette.nearest_colors_cache.contains_key(color) {
        return palette.colors[*palette.nearest_colors_cache.get(color).unwrap()];
    }
    let mut nearest_color_distance = std::u8::MAX as f32 * 3.0;
    let mut nearest_color_index = 0;
    let mut i = 0;
    for &palette_color in &palette.colors {
        let color_distance = colors_distance(&palette_color, &color);
        if color_distance < nearest_color_distance {
            nearest_color_distance = color_distance;
            nearest_color_index = i;
        }
        i += 1;
    }
    palette
        .nearest_colors_cache
        .insert(*color, nearest_color_index);
    return palette.colors[nearest_color_index];
}
