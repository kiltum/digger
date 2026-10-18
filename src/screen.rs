//! Indexed 640x400 software renderer.
//!
//! Two layers:
//! * `bg` — the destructible world (dirt pattern, dug tunnels, emeralds) and
//!   all static HUD text/title art.  Game logic mutates this layer through the
//!   same blob operations the original uses.
//! * `fb` — a per-frame copy of `bg` with the sprite records composed on top.
//!
//! Palette indices are kept all the way to presentation, which makes the
//! original palette/intensity flashing (`gpal`, `ginten`) trivial.

use std::sync::Arc;

use crate::assets::{Assets, Sprite};
use crate::constants::*;
use crate::palette;
use crate::sprites::SpriteSystem;

pub struct Screen {
    pub assets: Arc<Assets>,
    /// Pre-decoded render sprites for game indices 0..119.
    game_sprites: Vec<Option<Sprite>>,
    /// Destructible/static layer.
    pub bg: Vec<u8>,
    /// Composed frame (background + sprites), palette indices.
    fb: Vec<u8>,
    /// Final 0x00RRGGBB output.
    pub rgba: Vec<u32>,
    pub palette_index: u8,
    pub intensity: u8,
    /// The title bitmap was authored with its own palette.
    pub title_palette_active: bool,
    pub title_palette: [palette::Rgb; 16],
}

impl Screen {
    pub fn new(assets: Arc<Assets>) -> Self {
        let game_sprites = build_game_sprites(&assets);
        let title_palette = assets.title_palette;
        let mut s = Self {
            assets,
            game_sprites,
            bg: vec![0; FB_W * FB_H],
            fb: vec![0; FB_W * FB_H],
            rgba: vec![0; FB_W * FB_H],
            palette_index: 0,
            intensity: 0,
            title_palette_active: false,
            title_palette,
        };
        s.clear();
        s
    }

    pub fn clear(&mut self) {
        self.bg.fill(0);
        self.fb.fill(0);
        self.title_palette_active = false;
        self.palette_index = 0;
        self.intensity = 0;
    }

    pub fn set_palette(&mut self, index: u8) {
        self.palette_index = index;
        // Any palette change cancels the title's dedicated palette, matching
        // the original, where gpal() switches back to the VGA palette.
        self.title_palette_active = false;
    }

    pub fn set_intensity(&mut self, intensity: u8) {
        self.intensity = intensity;
        self.title_palette_active = false;
    }

    /// `gtitle()` — blit the title bitmap over the whole screen.
    pub fn gtitle(&mut self) {
        let t = &self.assets.title;
        let w = t.width.min(FB_W);
        let h = t.height.min(FB_H);
        for y in 0..h {
            let dst = y * FB_W;
            let src = y * t.width;
            self.bg[dst..dst + w].copy_from_slice(&t.pixels[src..src + w]);
        }
        // The bitmap's palette is the normal VGA palette (252 instead of 255
        // on a few entries), so the regular game palette is correct here —
        // this matches GetDIBits(..., windowed_palette[0], ...) in the
        // original Windows backend.
        self.title_palette_active = false;
    }

    // ------------------------------------------------------------------
    // Low-level blitting
    // ------------------------------------------------------------------

    /// Blit a decoded sprite to the static background.
    ///
    /// `w`/`h` are the logical dimensions used by the original draw call; the
    /// sprite data itself is at 2x, so only `w*8` by `h*2` physical pixels are
    /// copied.  0xff pixels are transparent.
    pub fn blit_bg(&mut self, x: i32, y: i32, sprite: &Sprite, w: i32, h: i32) {
        blit_indexed(&mut self.bg, x, y, sprite, w, h);
    }

    /// Blit a game sprite number to the background.
    pub fn draw_misc(&mut self, x: i32, y: i32, ch: usize, w: i32, h: i32) {
        let Some(sprite) = self.game_sprites.get(ch).and_then(|s| s.as_ref()) else {
            return;
        };
        if ch == 109 {
            erase_emerald_shape(&mut self.bg, x, y, sprite);
        } else {
            blit_indexed(&mut self.bg, x, y, sprite, w, h);
        }
    }

    /// Compose sprite `ch` over the frame (used by the sprite layer).
    pub fn blit_game(&mut self, assets: &Assets, x: i32, y: i32, ch: usize, w: i32, h: i32) {
        let _ = assets;
        let Some(sprite) = self.game_sprites.get(ch).and_then(|s| s.as_ref()) else {
            return;
        };
        if ch == 109 {
            erase_emerald_shape(&mut self.fb, x, y, sprite);
        } else {
            blit_indexed(&mut self.fb, x, y, sprite, w, h);
        }
    }

    /// `vgagetpix()` — sample the 4x1 logical-pixel box at (x,y) from the dirt
    /// layer.  Used by fireballs to detect walls.
    pub fn getpix(&self, x: i32, y: i32) -> i32 {
        if x < 0 || y < 0 || x > 319 || y > 199 {
            return 0xff;
        }
        let mut rval = 0i32;
        for yi in 0..2 {
            for xi in 0..8 {
                let px = x * 2 + xi;
                let py = y * 2 + yi;
                if self.bg[py as usize * FB_W + px as usize] != 0 {
                    rval |= 0x80 >> xi;
                }
            }
        }
        rval & 0xee
    }

    // ------------------------------------------------------------------
    // Playfield
    // ------------------------------------------------------------------

    /// `drawbackg()` — tile the dirt texture over the playfield.
    pub fn draw_background(&mut self, level: i32) {
        // The HUD lives above logical y=14 and is not touched.
        self.bg[28 * FB_W..].fill(0);
        let ch = 93 + level.clamp(1, 8) as usize;
        let mut y = 14;
        while y < 200 {
            let mut x = 0;
            while x < 320 {
                self.draw_misc(x, y, ch, 5, 4);
                x += 20;
            }
            y += 4;
        }
    }

    /// `drawfield()` — carve tunnels and edges out of the dirt pattern.
    pub fn draw_field(&mut self, field: &mut [u16; MSIZE]) {
        for x in 0..MWIDTH {
            for y in 0..MHEIGHT {
                let idx = y * MWIDTH + x;
                if field[idx] & 0x2000 != 0 {
                    continue;
                }
                let xp = x as i32 * 20 + 12;
                let yp = y as i32 * 18 + 18;
                if field[idx] & 0x0fc0 != 0x0fc0 {
                    field[idx] &= 0xd03f;
                    for k in 0..5 {
                        self.draw_bottom_blob(xp, yp - 15 + k * 3);
                    }
                    self.draw_top_blob(xp, yp + 3);
                }
                if field[idx] & 0x001f != 0x001f {
                    field[idx] &= 0xdfe0;
                    for k in 0..4 {
                        self.draw_right_blob(xp - 16 + k * 4, yp);
                    }
                    self.draw_left_blob(xp + 4, yp);
                }
                if x < MWIDTH - 1 && field[idx + 1] & 0x0fdf != 0x0fdf {
                    self.draw_right_blob(xp, yp);
                }
                if y < MHEIGHT - 1 && field[idx + MWIDTH] & 0x0fdf != 0x0fdf {
                    self.draw_bottom_blob(xp, yp);
                }
            }
        }
    }

    pub fn draw_right_blob(&mut self, x: i32, y: i32) {
        self.draw_misc(x + 16, y - 1, 102, 2, 18);
    }
    pub fn draw_left_blob(&mut self, x: i32, y: i32) {
        self.draw_misc(x - 8, y - 1, 104, 2, 18);
    }
    pub fn draw_top_blob(&mut self, x: i32, y: i32) {
        self.draw_misc(x - 4, y - 6, 103, 6, 6);
    }
    pub fn draw_bottom_blob(&mut self, x: i32, y: i32) {
        self.draw_misc(x - 4, y + 15, 105, 6, 6);
    }
    pub fn draw_furry_blob(&mut self, x: i32, y: i32) {
        self.draw_misc(x - 4, y + 15, 107, 6, 8);
    }
    pub fn draw_square_blob(&mut self, x: i32, y: i32) {
        self.draw_misc(x - 4, y + 17, 106, 6, 6);
    }

    pub fn draw_emerald(&mut self, x: i32, y: i32) {
        self.draw_misc(x, y, 108, 4, 10);
    }

    pub fn erase_emerald(&mut self, x: i32, y: i32) {
        self.draw_misc(x, y, 109, 4, 10);
    }

    // ------------------------------------------------------------------
    // Text
    // ------------------------------------------------------------------

    /// `gwrite()` — draw one 12x12 glyph at logical (x,y).
    pub fn gwrite(&mut self, x: i32, y: i32, ch: char, style: u8) {
        let dx = x * SCALE;
        let dy = y * SCALE;
        let Some(glyph) = self.assets.glyph(ch) else {
            fill_rect(&mut self.bg, dx, dy, 24, 24, 0);
            return;
        };
        let gw = glyph.width.min(24);
        let gh = glyph.height.min(24);
        for sy in 0..gh {
            let py = dy + sy as i32;
            if py < 0 || py >= FB_H as i32 {
                continue;
            }
            for sx in 0..gw {
                let px = dx + sx as i32;
                if px < 0 || px >= FB_W as i32 {
                    continue;
                }
                let mut c = glyph.pixels[sy * glyph.width + sx];
                if c == 0xff {
                    continue;
                }
                c = palette::text_color(c, style);
                self.bg[py as usize * FB_W + px as usize] = c;
            }
        }
    }

    /// `outtext()` — advances 12 logical pixels per character.
    pub fn outtext(&mut self, text: &str, mut x: i32, y: i32, style: u8) {
        for ch in text.chars() {
            self.gwrite(x, y, ch, style);
            x += 12;
        }
    }

    /// `writenum()` — right-aligned fixed-width number.
    pub fn writenum(&mut self, n: i64, x: i32, y: i32, width: i32, style: u8) {
        let n = n.rem_euclid(1_000_000);
        let mut xp = (width - 1) * 12 + x;
        let mut n = n;
        let mut w = width;
        while w > 0 {
            let d = (n % 10) as u8;
            if w > 1 || d > 0 {
                self.gwrite(xp, y, (b'0' + d) as char, style);
            }
            n /= 10;
            w -= 1;
            xp -= 12;
        }
    }

    // ------------------------------------------------------------------
    // Presentation
    // ------------------------------------------------------------------

    /// Compose background + sprites and convert to 0x00RRGGBB.
    pub fn compose(&mut self, sprites: &SpriteSystem) {
        self.fb.copy_from_slice(&self.bg);
        for rec in sprites.recs.iter() {
            if rec.enabled {
                let Some(sprite) = self
                    .game_sprites
                    .get(rec.ch as usize)
                    .and_then(|s| s.as_ref())
                else {
                    continue;
                };
                if rec.ch == 109 {
                    erase_emerald_shape(&mut self.fb, rec.x, rec.y, sprite);
                } else {
                    blit_indexed(&mut self.fb, rec.x, rec.y, sprite, rec.w, rec.h);
                }
            }
        }

        let pal: &[palette::Rgb; 16] = if self.title_palette_active {
            &self.title_palette
        } else {
            palette::palette(self.palette_index, self.intensity)
        };
        for (i, &idx) in self.fb.iter().enumerate() {
            let [r, g, b] = pal[(idx as usize).min(15)];
            self.rgba[i] = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        }
    }

    pub fn frame(&self) -> &[u32] {
        &self.rgba
    }
}

/// Build the 120 game-facing render sprites using the asset layer's resolved
/// form (this is important for `BlackMask` indices such as the blob masks and
/// empty life slots, and for `EraseEmerald`).
fn build_game_sprites(assets: &Assets) -> Vec<Option<Sprite>> {
    (0..crate::assets::GAME_SPRITE_COUNT)
        .map(|ch| assets.sprite(ch).cloned())
        .collect()
}

/// Paint black over the opaque part of the emerald glyph (`vgazero480` +
/// `vgaemeraldmask` in the original).
fn erase_emerald_shape(dst: &mut [u8], x: i32, y: i32, emerald: &Sprite) {
    let dx = x * SCALE;
    let dy = y * SCALE;
    for sy in 0..emerald.height {
        let py = dy + sy as i32;
        if py < 0 || py >= FB_H as i32 {
            continue;
        }
        for sx in 0..emerald.width {
            let px = dx + sx as i32;
            if px < 0 || px >= FB_W as i32 {
                continue;
            }
            if emerald.pixels[sy * emerald.width + sx] != 0xff {
                dst[py as usize * FB_W + px as usize] = 0;
            }
        }
    }
}

fn blit_indexed(dst: &mut [u8], x: i32, y: i32, sprite: &Sprite, w: i32, h: i32) {
    let dx = x * SCALE;
    let dy = y * SCALE;
    let clip_w = (w * 8).max(0).min(sprite.width as i32);
    let clip_h = (h * 2).max(0).min(sprite.height as i32);
    for sy in 0..clip_h {
        let py = dy + sy;
        if py < 0 || py >= FB_H as i32 {
            continue;
        }
        for sx in 0..clip_w {
            let px = dx + sx;
            if px < 0 || px >= FB_W as i32 {
                continue;
            }
            let c = sprite.pixels[sy as usize * sprite.width + sx as usize];
            // 0xff is the only transparent marker; palette index 0 (black)
            // must be drawn, because the tunnel/blob sprites are black masks.
            if c != 0xff {
                dst[py as usize * FB_W + px as usize] = c;
            }
        }
    }
}

fn fill_rect(dst: &mut [u8], x: i32, y: i32, w: i32, h: i32, color: u8) {
    for yy in y.max(0)..(y + h).min(FB_H as i32) {
        let start = yy as usize * FB_W + x.max(0) as usize;
        let end = yy as usize * FB_W + (x + w).min(FB_W as i32) as usize;
        if start < end {
            dst[start..end].fill(color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite2x2() -> Sprite {
        // top-left black (opaque), top-right magenta, bottom-left transparent,
        // bottom-right white.
        Sprite {
            width: 2,
            height: 2,
            pixels: vec![0, 13, 0xff, 15],
        }
    }

    #[test]
    fn zero_is_opaque_and_ff_is_transparent() {
        let mut dst = vec![7u8; FB_W * FB_H];
        // w/h are game units: w=1 -> 8 physical px, h=1 -> 2 rows. The source
        // sprite is 2x2, so only the 2x2 top-left is used, repeated over the
        // 8x2 clip.
        blit_indexed(&mut dst, 0, 0, &sprite2x2(), 1, 1);
        assert_eq!(dst[0], 0, "palette index 0 must be drawn as black");
        assert_eq!(dst[1], 13, "colour pixel drawn");
        assert_eq!(dst[FB_W], 7, "0xff must be transparent");
        assert_eq!(dst[FB_W + 1], 15);
        // The 8px-wide clip mirrors the 2px source only for sx<2; sx>=2 reads
        // out of the 2x2 sprite and is skipped by clip_w = min(w*8, width).
        assert_eq!(dst[2], 7);
    }

    #[test]
    fn erase_emerald_covers_the_full_sprite() {
        let mut dst = vec![5u8; FB_W * FB_H];
        let emerald = Sprite {
            width: 32,
            height: 20,
            pixels: vec![0; 32 * 20], // fully opaque artwork
        };
        erase_emerald_shape(&mut dst, 10, 10, &emerald);
        // Physical destination starts at (20,20), 32x20 pixels wide/tall.
        assert_eq!(dst[20 * FB_W + 20], 0);
        assert_eq!(dst[20 * FB_W + 20 + 31], 0);
        assert_eq!(dst[(20 + 19) * FB_W + 20], 0);
        assert_eq!(dst[(20 + 19) * FB_W + 20 + 31], 0);
    }
}
