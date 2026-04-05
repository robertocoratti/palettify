use image::{DynamicImage, RgbImage};

use crate::color::{linear_to_srgb, rgb_to_oklab, srgb_to_linear};
use crate::palette::Palette;

pub(super) fn process(img: DynamicImage, palette: &Palette) -> RgbImage {
    let img = img.to_rgb8();
    let w = img.width() as usize;
    let h = img.height() as usize;

    // Working buffer in linear RGB; quantisation error accumulates here.
    let mut buf: Vec<[f32; 3]> = img
        .pixels()
        .map(|p| {
            let [r, g, b] = p.0;
            [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)]
        })
        .collect();

    let mut raw = vec![0u8; w * h * 3];

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let [lr, lg, lb] = buf[idx];

            // Clamp accumulated linear value, convert to sRGB u8 for palette lookup.
            let cr = linear_to_srgb(lr);
            let cg = linear_to_srgb(lg);
            let cb = linear_to_srgb(lb);

            let lab = rgb_to_oklab(cr, cg, cb);
            let nearest = palette.nearest(&lab);

            let out = idx * 3;
            raw[out]     = nearest.r;
            raw[out + 1] = nearest.g;
            raw[out + 2] = nearest.b;

            // Quantisation error in linear RGB.
            let err = [
                lr - srgb_to_linear(nearest.r),
                lg - srgb_to_linear(nearest.g),
                lb - srgb_to_linear(nearest.b),
            ];

            // Distribute error to four neighbours (Floyd-Steinberg weights):
            //         [*] [7/16]
            // [3/16] [5/16] [1/16]
            macro_rules! add_err {
                ($ni:expr, $w:expr) => {
                    buf[$ni][0] += err[0] * $w;
                    buf[$ni][1] += err[1] * $w;
                    buf[$ni][2] += err[2] * $w;
                };
            }

            if x + 1 < w               { add_err!(idx + 1,               7.0 / 16.0); }
            if y + 1 < h && x > 0      { add_err!((y + 1) * w + x - 1,  3.0 / 16.0); }
            if y + 1 < h               { add_err!((y + 1) * w + x,       5.0 / 16.0); }
            if y + 1 < h && x + 1 < w  { add_err!((y + 1) * w + x + 1,  1.0 / 16.0); }
        }
    }

    RgbImage::from_raw(w as u32, h as u32, raw)
        .expect("pixel buffer size mismatch — this is a bug in floyd_steinberg::process")
}
