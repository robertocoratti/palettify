/// Palette files embedded into the binary at compile time via include_str!.
///
/// These serve as a guaranteed fallback so the server always starts with a
/// full set of palettes, even when PALETTES_DIR is not configured or missing.
/// When PALETTES_DIR is set, files on disk take precedence and can add or
/// override entries in this list.
pub const EMBEDDED: &[(&str, &str)] = &[
    ("catppuccin-mocha", include_str!("../palettes/catppuccin-mocha.txt")),
    ("catppuccin-latte", include_str!("../palettes/catppuccin-latte.txt")),
    ("dracula",          include_str!("../palettes/dracula.txt")),
    ("gruvbox-dark",     include_str!("../palettes/gruvbox-dark.txt")),
    ("gruvbox-light",    include_str!("../palettes/gruvbox-light.txt")),
    ("nord",             include_str!("../palettes/nord.txt")),
    ("rose-pine",        include_str!("../palettes/rose-pine.txt")),
    ("solarized-dark",   include_str!("../palettes/solarized-dark.txt")),
    ("tokyo-night",      include_str!("../palettes/tokyo-night.txt")),
    ("one-dark",         include_str!("../palettes/one-dark.txt")),
];
