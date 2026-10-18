//! VGA palettes and text-colour remapping.
//!
//! Ported from Digger Remastered's `win_vid.c`:
//!
//! * `vga16_pal1_rgbq`, `vga16_pal1i_rgbq`, `vga16_pal2_rgbq`,
//!   `vga16_pal2i_rgbq` — the four 16-colour VGA palettes.
//! * `vgawrite()` — the colour remapping applied while drawing text.
//!
//! A Windows `RGBQUAD` stores its bytes in `B, G, R, reserved` order, so the
//! tables below are the `win_vid.c` literals converted to `[r, g, b]`.

/// A single RGB colour, one byte per channel.
pub type Rgb = [u8; 3];

/// Palette 1, normal intensity (`vga16_pal1_rgbq`).
pub const PAL1_NORMAL: [Rgb; 16] = [
    [0, 0, 0],
    [0, 0, 128],
    [0, 128, 0],
    [0, 128, 128],
    [128, 0, 0],
    [128, 0, 128],
    [128, 64, 0],
    [128, 128, 128],
    [64, 64, 64],
    [0, 0, 255],
    [0, 255, 0],
    [0, 255, 255],
    [255, 0, 0],
    [255, 0, 255],
    [255, 255, 0],
    [255, 255, 255],
];

/// Palette 1, high intensity (`vga16_pal1i_rgbq`).
pub const PAL1_INTENSE: [Rgb; 16] = [
    [0, 0, 0],
    [0, 0, 255],
    [0, 255, 0],
    [0, 255, 255],
    [255, 0, 0],
    [255, 0, 255],
    [255, 128, 0],
    [192, 192, 192],
    [128, 128, 128],
    [128, 128, 255],
    [128, 255, 128],
    [128, 255, 255],
    [255, 128, 128],
    [255, 128, 255],
    [255, 255, 128],
    [255, 255, 255],
];

/// Palette 2, normal intensity (`vga16_pal2_rgbq`).
pub const PAL2_NORMAL: [Rgb; 16] = [
    [0, 0, 0],
    [0, 128, 0],
    [128, 0, 0],
    [128, 64, 0],
    [0, 0, 128],
    [0, 128, 128],
    [128, 0, 128],
    [128, 128, 128],
    [64, 64, 64],
    [0, 255, 0],
    [255, 0, 0],
    [255, 255, 0],
    [0, 0, 255],
    [0, 255, 255],
    [255, 0, 255],
    [255, 255, 255],
];

/// Palette 2, high intensity (`vga16_pal2i_rgbq`).
pub const PAL2_INTENSE: [Rgb; 16] = [
    [0, 0, 0],
    [0, 255, 0],
    [255, 0, 0],
    [255, 128, 0],
    [0, 0, 255],
    [0, 255, 255],
    [255, 0, 255],
    [192, 192, 192],
    [128, 128, 128],
    [128, 255, 128],
    [255, 128, 128],
    [255, 255, 128],
    [128, 128, 255],
    [128, 255, 255],
    [255, 128, 255],
    [255, 255, 255],
];

/// Select one of the four palettes.
///
/// `index` 0 selects palette 1 and 1 selects palette 2 (matching the game's
/// `vgapal`/`vgainten` state); any other value falls back to palette 2.
/// `intensity` 0 is normal, 1 is high intensity (any non-zero value is
/// treated as high intensity).
pub fn palette(index: u8, intensity: u8) -> &'static [Rgb; 16] {
    match (index, intensity) {
        (0, 0) => &PAL1_NORMAL,
        (0, _) => &PAL1_INTENSE,
        (_, 0) => &PAL2_NORMAL,
        (_, _) => &PAL2_INTENSE,
    }
}

/// Remap a font colour index for the current text style.
///
/// This is the exact transformation performed by `vgawrite()` in
/// `win_vid.c`:
///
/// * colour 10 becomes 12 in style 2 and 14 in style 3;
/// * colour 12 becomes 2, 4 or 6 in styles 1, 2 or 3;
/// * every other colour is unchanged.
pub fn text_color(color: u8, style: u8) -> u8 {
    match color {
        10 => match style {
            2 => 12,
            3 => 14,
            _ => 10,
        },
        12 => match style {
            1 => 2,
            2 => 4,
            3 => 6,
            _ => 12,
        },
        _ => color,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette1_normal_values() {
        // RGBQUAD {128,0,0,0} is blue in [r,g,b] order.
        assert_eq!(PAL1_NORMAL[0], [0, 0, 0]);
        assert_eq!(PAL1_NORMAL[1], [0, 0, 128]);
        assert_eq!(PAL1_NORMAL[2], [0, 128, 0]);
        assert_eq!(PAL1_NORMAL[3], [0, 128, 128]);
        assert_eq!(PAL1_NORMAL[4], [128, 0, 0]);
        assert_eq!(PAL1_NORMAL[6], [128, 64, 0]);
        assert_eq!(PAL1_NORMAL[9], [0, 0, 255]);
        assert_eq!(PAL1_NORMAL[12], [255, 0, 0]);
        assert_eq!(PAL1_NORMAL[15], [255, 255, 255]);
    }

    #[test]
    fn palette1_intense_values() {
        assert_eq!(PAL1_INTENSE[1], [0, 0, 255]);
        assert_eq!(PAL1_INTENSE[6], [255, 128, 0]);
        assert_eq!(PAL1_INTENSE[7], [192, 192, 192]);
        assert_eq!(PAL1_INTENSE[9], [128, 128, 255]);
        assert_eq!(PAL1_INTENSE[12], [255, 128, 128]);
        assert_eq!(PAL1_INTENSE[14], [255, 255, 128]);
    }

    #[test]
    fn palette2_values() {
        assert_eq!(PAL2_NORMAL[1], [0, 128, 0]);
        assert_eq!(PAL2_NORMAL[2], [128, 0, 0]);
        assert_eq!(PAL2_NORMAL[3], [128, 64, 0]);
        assert_eq!(PAL2_NORMAL[13], [0, 255, 255]);
        assert_eq!(PAL2_INTENSE[1], [0, 255, 0]);
        assert_eq!(PAL2_INTENSE[2], [255, 0, 0]);
        assert_eq!(PAL2_INTENSE[13], [128, 255, 255]);
        assert_eq!(PAL2_INTENSE[14], [255, 128, 255]);
    }

    #[test]
    fn palette_selection() {
        assert_eq!(*palette(0, 0), PAL1_NORMAL);
        assert_eq!(*palette(0, 1), PAL1_INTENSE);
        assert_eq!(*palette(1, 0), PAL2_NORMAL);
        assert_eq!(*palette(1, 1), PAL2_INTENSE);
        // Out-of-range values fall back to palette 2.
        assert_eq!(*palette(2, 0), PAL2_NORMAL);
        assert_eq!(*palette(2, 1), PAL2_INTENSE);
        // Any non-zero intensity selects the intense palette.
        assert_eq!(*palette(0, 7), PAL1_INTENSE);
    }

    #[test]
    fn text_color_remapping() {
        // Colour 10 special cases.
        assert_eq!(text_color(10, 0), 10);
        assert_eq!(text_color(10, 1), 10);
        assert_eq!(text_color(10, 2), 12);
        assert_eq!(text_color(10, 3), 14);
        assert_eq!(text_color(10, 4), 10);

        // Colour 12 special cases.
        assert_eq!(text_color(12, 0), 12);
        assert_eq!(text_color(12, 1), 2);
        assert_eq!(text_color(12, 2), 4);
        assert_eq!(text_color(12, 3), 6);
        assert_eq!(text_color(12, 4), 12);

        // All other colours are unchanged.
        for color in 0..=15 {
            if color != 10 && color != 12 {
                for style in 0..=4 {
                    assert_eq!(text_color(color, style), color);
                }
            }
        }
    }
}
