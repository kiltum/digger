//! Graphics asset loading for the VGA build of Digger Remastered.
//!
//! The three assets used by the renderer are embedded at compile time:
//!
//! * `vga.spr`     — 107 raw sprite records of 1024 bytes each, decoded the
//!                   same way as `mkgv()` in `ref/remastered/mkg.c`.
//! * `vgatext.spr` — 40 24x24 font glyphs in `clist[]` order.
//! * `vtitle.bmp`  — the 640x400 4bpp RLE4 title bitmap, decoded by the small
//!                   RLE4 decoder below.
//!
//! Game code addresses sprites with an index in `0..=119` (the order of
//! `vgatable[]` in `ref/windows/vgagrafx.c`); [`game_sprite_raw_index`] maps
//! those indices onto the raw `.spr` records.

use crate::palette::{self, Rgb};

/// Number of game sprite indices (`vgatable[]` entries), `0..=119`.
pub const GAME_SPRITE_COUNT: usize = 120;

/// Number of records in `vga.spr` (the `gnames[]` table in `mkg.c`).
pub const RAW_SPRITE_COUNT: usize = 107;

/// Number of glyphs in `vgatext.spr` (`clist[]` in `mkg.c`).
pub const TEXT_GLYPH_COUNT: usize = 40;

/// Every `vga.spr` record is exactly 1024 bytes.
pub const SPRITE_RECORD_SIZE: usize = 1024;

/// A raw sprite record is 32 bytes per source row.
const RECORD_ROW_BYTES: usize = 32;

const VGA_SPR: &[u8] = include_bytes!("../assets/vga.spr");
const VGA_TEXT_SPR: &[u8] = include_bytes!("../assets/vgatext.spr");
const V_TITLE_BMP: &[u8] = include_bytes!("../assets/vtitle.bmp");

/// Glyph order in `vgatext.spr` (`char clist[]` in `mkg.c`).
const CHARSET: &[u8; TEXT_GLYPH_COUNT] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.:_ ";

/// `widths[]` from `mkg.c`, in 8-pixel units.
const WIDTH_UNITS: [usize; RAW_SPRITE_COUNT] = [
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 2, 2, 6, 6, 6, 6, 4, 5, 5, 5, 5, 5, 5, 5, 5, 2,
    2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
];

/// `heights[]` from `mkg.c`, in 2-pixel units.
const HEIGHT_UNITS: [usize; RAW_SPRITE_COUNT] = [
    10, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 14, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 12, 12, 12, 12, 18, 18, 6, 6, 8, 6, 15, 4, 4, 4, 4, 4, 4, 4, 4, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8,
];

/// A decoded sprite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sprite {
    pub width: usize,
    pub height: usize,
    /// Palette indices, one byte per pixel. `0xff` means "transparent" for
    /// normal (colour) sprites; mask sprites use `0x00`/`0x0f`.
    pub pixels: Vec<u8>,
}

/// How a game sprite index (`vgatable[]`) resolves against the raw assets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameSprite {
    /// A normal raw record from `vga.spr`.
    Raw(usize),
    /// Draws nothing (`vgazero480` with no visible effect).
    Blank,
    /// The erase-emerald sprite: paint black over the emerald's opaque
    /// pixels. [`Assets::sprite`] returns the emerald artwork for it.
    EraseEmerald,
    /// A mask-only sprite (`vgazero480` data plus a mask). The mask sprites
    /// paint black where the mask is clear and leave the screen untouched
    /// where it is set. [`Assets::sprite`] returns the game-facing sprite:
    /// `0x00` (black) where the mask is `0x00`, `0xff` (transparent) where the
    /// mask is `0x0f`.
    BlackMask(usize),
}

/// The game-facing `vgatable[]` mapping, derived from `vgagrafx.c`.
///
/// Each entry pairs the game sprite index (`0..=119`) with the raw record it
/// draws. The comments name the `gnames[]` entry (or special case).
const GAME_SPRITES: [GameSprite; GAME_SPRITE_COUNT] = [
    GameSprite::Blank,         //   0  zero + edigger mask (blank placeholder)
    GameSprite::Raw(25),       //   1  rdigger1
    GameSprite::Raw(23),       //   2  rdigger2
    GameSprite::Raw(21),       //   3  rdigger3
    GameSprite::Raw(26),       //   4  rxdigger1
    GameSprite::Raw(24),       //   5  rxdigger2
    GameSprite::Raw(22),       //   6  rxdigger3
    GameSprite::Raw(37),       //   7  udigger1
    GameSprite::Raw(35),       //   8  udigger2
    GameSprite::Raw(33),       //   9  udigger3
    GameSprite::Raw(38),       //  10  uxdigger1
    GameSprite::Raw(36),       //  11  uxdigger2
    GameSprite::Raw(34),       //  12  uxdigger3
    GameSprite::Raw(31),       //  13  ldigger1
    GameSprite::Raw(29),       //  14  ldigger2
    GameSprite::Raw(27),       //  15  ldigger3
    GameSprite::Raw(32),       //  16  lxdigger1
    GameSprite::Raw(30),       //  17  lxdigger2
    GameSprite::Raw(28),       //  18  lxdigger3
    GameSprite::Raw(43),       //  19  ddigger1
    GameSprite::Raw(41),       //  20  ddigger2
    GameSprite::Raw(39),       //  21  ddigger3
    GameSprite::Raw(44),       //  22  dxdigger1
    GameSprite::Raw(42),       //  23  dxdigger2
    GameSprite::Raw(40),       //  24  dxdigger3
    GameSprite::Raw(74),       //  25  diggerd
    GameSprite::Raw(73),       //  26  grave1
    GameSprite::Raw(72),       //  27  grave2
    GameSprite::Raw(71),       //  28  grave3
    GameSprite::Raw(70),       //  29  grave4
    GameSprite::Raw(69),       //  30  grave5
    GameSprite::Blank,         //  31  zero + edigger mask (blank placeholder)
    GameSprite::Raw(49),       //  32  rbigger1
    GameSprite::Raw(47),       //  33  rbigger2
    GameSprite::Raw(45),       //  34  rbigger3
    GameSprite::Raw(50),       //  35  rxbigger1
    GameSprite::Raw(48),       //  36  rxbigger2
    GameSprite::Raw(46),       //  37  rxbigger3
    GameSprite::Raw(61),       //  38  ubigger1
    GameSprite::Raw(59),       //  39  ubigger2
    GameSprite::Raw(57),       //  40  ubigger3
    GameSprite::Raw(62),       //  41  uxbigger1
    GameSprite::Raw(60),       //  42  uxbigger2
    GameSprite::Raw(58),       //  43  uxbigger3
    GameSprite::Raw(55),       //  44  lbigger1
    GameSprite::Raw(53),       //  45  lbigger2
    GameSprite::Raw(51),       //  46  lbigger3
    GameSprite::Raw(56),       //  47  lxbigger1
    GameSprite::Raw(54),       //  48  lxbigger2
    GameSprite::Raw(52),       //  49  lxbigger3
    GameSprite::Raw(67),       //  50  dbigger1
    GameSprite::Raw(65),       //  51  dbigger2
    GameSprite::Raw(63),       //  52  dbigger3
    GameSprite::Raw(68),       //  53  dxbigger1
    GameSprite::Raw(66),       //  54  dxbigger2
    GameSprite::Raw(64),       //  55  dxbigger3
    GameSprite::Raw(75),       //  56  biggerd
    GameSprite::Raw(73),       //  57  grave1
    GameSprite::Raw(72),       //  58  grave2
    GameSprite::Raw(71),       //  59  grave3
    GameSprite::Raw(70),       //  60  grave4
    GameSprite::Raw(69),       //  61  grave5
    GameSprite::Raw(1),        //  62  sbag
    GameSprite::Raw(2),        //  63  rbag
    GameSprite::Raw(3),        //  64  lbag
    GameSprite::Raw(4),        //  65  fbag
    GameSprite::Raw(5),        //  66  gold1
    GameSprite::Raw(6),        //  67  gold2
    GameSprite::Raw(7),        //  68  gold3
    GameSprite::Raw(10),       //  69  nobbin1
    GameSprite::Raw(9),        //  70  nobbin2
    GameSprite::Raw(8),        //  71  nobbin3
    GameSprite::Raw(11),       //  72  nobbind
    GameSprite::Raw(12),       //  73  rhobbin1
    GameSprite::Raw(13),       //  74  rhobbin2
    GameSprite::Raw(14),       //  75  rhobbin3
    GameSprite::Raw(15),       //  76  rhobbind
    GameSprite::Raw(16),       //  77  lhobbin1
    GameSprite::Raw(17),       //  78  lhobbin2
    GameSprite::Raw(18),       //  79  lhobbin3
    GameSprite::Raw(19),       //  80  lhobbind
    GameSprite::Raw(20),       //  81  bonus
    GameSprite::Raw(95),       //  82  fire1
    GameSprite::Raw(96),       //  83  fire2
    GameSprite::Raw(97),       //  84  fire3
    GameSprite::Raw(98),       //  85  exp1
    GameSprite::Raw(99),       //  86  exp2
    GameSprite::Raw(100),      //  87  exp3
    GameSprite::Raw(95),       //  88  fire1 (player-0 duplicate)
    GameSprite::Raw(96),       //  89  fire2 (player-0 duplicate)
    GameSprite::Raw(97),       //  90  fire3 (player-0 duplicate)
    GameSprite::Raw(98),       //  91  exp1  (player-0 duplicate)
    GameSprite::Raw(99),       //  92  exp2  (player-0 duplicate)
    GameSprite::Raw(100),      //  93  exp3  (player-0 duplicate)
    GameSprite::Raw(87),       //  94  back1
    GameSprite::Raw(88),       //  95  back2
    GameSprite::Raw(89),       //  96  back3
    GameSprite::Raw(90),       //  97  back4
    GameSprite::Raw(91),       //  98  back5
    GameSprite::Raw(92),       //  99  back6
    GameSprite::Raw(93),       // 100  back7
    GameSprite::Raw(94),       // 101  back8
    GameSprite::BlackMask(80), // 102  zero + rightblob mask
    GameSprite::BlackMask(82), // 103  zero + topblob mask
    GameSprite::BlackMask(81), // 104  zero + leftblob mask
    GameSprite::BlackMask(83), // 105  zero + bottomblob mask
    GameSprite::BlackMask(85), // 106  zero + squareblob mask
    GameSprite::BlackMask(84), // 107  zero + furryblob mask
    GameSprite::Raw(0),        // 108  emerald
    GameSprite::EraseEmerald,  // 109  zero + emerald mask
    GameSprite::Raw(76),       // 110  liferight
    GameSprite::Raw(77),       // 111  lifeleft
    GameSprite::BlackMask(78), // 112  zero + elife mask (solid black erase)
    GameSprite::Raw(79),       // 113  leftlife
    GameSprite::Raw(101),      // 114  fire1b
    GameSprite::Raw(102),      // 115  fire2b
    GameSprite::Raw(103),      // 116  fire3b
    GameSprite::Raw(104),      // 117  exp1b
    GameSprite::Raw(105),      // 118  exp2b
    GameSprite::Raw(106),      // 119  exp3b
];

/// Map a game sprite index (`vgatable[]` order, `0..=119`) to its raw asset.
///
/// Returns `None` for indices outside the game's sprite range.
pub fn game_sprite_raw_index(ch: usize) -> Option<GameSprite> {
    GAME_SPRITES.get(ch).copied()
}

/// All graphics needed by the renderer.
pub struct Assets {
    /// The 107 raw records from `vga.spr`, in `gnames[]` order. Mask-only
    /// records (`78` and `80..=86`) are stored as `0x00`/`0x0f` masks.
    pub sprites: Vec<Sprite>,
    /// The 40 glyphs from `vgatext.spr`, 24x24 each, in `clist[]` order.
    pub text: Vec<Sprite>,
    /// The 640x400 title bitmap, top-down row-major palette indices.
    pub title: Sprite,
    /// The title bitmap's own 16-colour palette.
    pub title_palette: [palette::Rgb; 16],
    /// Derived game-facing sprites for [`GameSprite::BlackMask`]. Kept
    /// private so [`Assets::sprite`] can hand out references.
    black_masks: Vec<Option<Sprite>>,
}

impl Assets {
    /// Decode the embedded graphics.
    pub fn load() -> Result<Assets, String> {
        if VGA_SPR.len() != RAW_SPRITE_COUNT * SPRITE_RECORD_SIZE {
            return Err(format!(
                "vga.spr: expected {} bytes ({} records of {}), got {}",
                RAW_SPRITE_COUNT * SPRITE_RECORD_SIZE,
                RAW_SPRITE_COUNT,
                SPRITE_RECORD_SIZE,
                VGA_SPR.len()
            ));
        }

        let mut sprites = Vec::with_capacity(RAW_SPRITE_COUNT);
        for i in 0..RAW_SPRITE_COUNT {
            let start = i * SPRITE_RECORD_SIZE;
            let record = &VGA_SPR[start..start + SPRITE_RECORD_SIZE];
            sprites.push(decode_raw_sprite(record, i)?);
        }

        let text = decode_text(VGA_TEXT_SPR)?;
        let (title, title_palette) = decode_title(V_TITLE_BMP)?;
        if title.width != 640 || title.height != 400 {
            return Err(format!(
                "vtitle.bmp: expected 640x400, got {}x{}",
                title.width, title.height
            ));
        }

        // Mask-only records (the six blob masks plus edigger/elife) are
        // already stored in mask form by `decode_raw_sprite`.
        let mut black_masks: Vec<Option<Sprite>> = vec![None; RAW_SPRITE_COUNT];
        for &i in &[78usize, 80, 81, 82, 83, 84, 85, 86] {
            black_masks[i] = Some(black_from_mask(&sprites[i]));
        }

        Ok(Assets {
            sprites,
            text,
            title,
            title_palette,
            black_masks,
        })
    }

    /// Map a game sprite index to its raw-asset description.
    pub fn game_sprite_ref(&self, ch: usize) -> Option<GameSprite> {
        game_sprite_raw_index(ch)
    }

    /// Resolve a game sprite index to drawable pixels.
    ///
    /// * `Raw(i)` returns raw record `i`.
    /// * `BlackMask(i)` returns record `i` converted to black/transparent.
    /// * `EraseEmerald` returns the emerald artwork (the caller paints black
    ///   wherever it is opaque).
    /// * `Blank` returns `None`.
    pub fn sprite(&self, ch: usize) -> Option<&Sprite> {
        match self.game_sprite_ref(ch)? {
            GameSprite::Raw(i) => self.sprites.get(i),
            GameSprite::BlackMask(i) => self.black_masks.get(i).and_then(|m| m.as_ref()),
            GameSprite::EraseEmerald => self.sprites.first(),
            GameSprite::Blank => None,
        }
    }

    /// Look up a font glyph. Letters are case-insensitive; only the
    /// characters in `clist[]` exist.
    pub fn glyph(&self, ch: char) -> Option<&Sprite> {
        let upper = ch.to_ascii_uppercase();
        if !upper.is_ascii() {
            return None;
        }
        let index = CHARSET.iter().position(|&c| c == upper as u8)?;
        self.text.get(index)
    }
}

fn byte_at(record: &[u8], index: usize) -> Result<u8, String> {
    record
        .get(index)
        .copied()
        .ok_or_else(|| format!("sprite record too short: byte {index} out of range"))
}

/// Decode one 1024-byte `vga.spr` record.
///
/// This mirrors the four branches of `mkgv()` in `mkg.c`:
///
/// * normal colour data (`i < 80 || i >= 95`);
/// * background tiles (`87..=94`) with their two-part 40x8 layout;
/// * tall masks (`80..=81`) and wide masks (`82..=85`) built from the low
///   nibble of each source byte;
/// * the normal (bit 7) mask used for `elife` (78) and `edigger` (86).
///
/// Normal sprites are `widths[i]*8` by `heights[i]*2`; the source byte for
/// pixel `(x, y)` is `record[y*32 + x]` (which may cross the nominal 32-byte
/// row, exactly as in `mkgv()`). A `0xff` byte is transparent.
fn decode_raw_sprite(record: &[u8], index: usize) -> Result<Sprite, String> {
    if record.len() != SPRITE_RECORD_SIZE {
        return Err(format!(
            "sprite {index}: expected {SPRITE_RECORD_SIZE} bytes, got {}",
            record.len()
        ));
    }

    let width = WIDTH_UNITS[index] * 8;
    let height = HEIGHT_UNITS[index] * 2;

    // Background tiles: mkgv() reads the first 32 pixels from the first
    // half of each row and the remaining 8 from `record[y*32 + 512 ..]`.
    if (87..=94).contains(&index) {
        let mut pixels = Vec::with_capacity(40 * 8);
        for y in 0..8 {
            for x in 0..40 {
                let offset = if x < 32 {
                    y * RECORD_ROW_BYTES + x
                } else {
                    y * RECORD_ROW_BYTES + 512 + (x - 32)
                };
                pixels.push(byte_at(record, offset)?);
            }
        }
        return Ok(Sprite {
            width: 40,
            height: 8,
            pixels,
        });
    }

    // Tall masks (rightblob, leftblob): 16x36. The first 32 rows come from
    // bytes 0..16 of each source row, the last 4 from bytes 16..32 of rows
    // 0..4 (`mkgv()`'s "Tall masks" branch).
    if index == 80 || index == 81 {
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..32 {
            for x in 0..width {
                let byte = byte_at(record, y * RECORD_ROW_BYTES + x)?;
                pixels.push(if byte & 0x0f != 0 { 0x0f } else { 0 });
            }
        }
        for y in 0..4 {
            for x in 0..width {
                let byte = byte_at(record, y * RECORD_ROW_BYTES + 16 + x)?;
                pixels.push(if byte & 0x0f != 0 { 0x0f } else { 0 });
            }
        }
        return Ok(Sprite {
            width,
            height,
            pixels,
        });
    }

    // Wide masks (topblob, bottomblob, furryblob, squareblob): 48 pixels
    // wide. The first 32 pixels come from the current row, the last 16 from
    // row `y + 16` (`mkgv()`'s "Wide masks" branch).
    if (82..=85).contains(&index) {
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..32 {
                let byte = byte_at(record, y * RECORD_ROW_BYTES + x)?;
                pixels.push(if byte & 0x0f != 0 { 0x0f } else { 0 });
            }
            for x in 0..16 {
                let byte = byte_at(record, (y + 16) * RECORD_ROW_BYTES + x)?;
                pixels.push(if byte & 0x0f != 0 { 0x0f } else { 0 });
            }
        }
        return Ok(Sprite {
            width,
            height,
            pixels,
        });
    }

    // edigger (86) and elife (78) only have a normal mask: bit 7 of each
    // source byte. Both are used by the game with `vgazero480` data, so the
    // stored form is the mask rather than the (unused) low-nibble artwork.
    if index == 78 || index == 86 {
        return normal_mask_sprite(record, width, height);
    }

    // Ordinary colour sprite: one palette index per byte.
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            pixels.push(byte_at(record, y * RECORD_ROW_BYTES + x)?);
        }
    }
    Ok(Sprite {
        width,
        height,
        pixels,
    })
}

/// Build the "normal mask" from `mkgv()`'s normal-mask branch: the mask bit
/// is bit 7 (the `0xff` transparency bit) of each source byte. The result is
/// `0x0f` where the pixel is transparent and `0x00` where it is opaque.
fn normal_mask_sprite(record: &[u8], width: usize, height: usize) -> Result<Sprite, String> {
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let byte = byte_at(record, y * RECORD_ROW_BYTES + x)?;
            pixels.push(if byte & 0x80 != 0 { 0x0f } else { 0 });
        }
    }
    Ok(Sprite {
        width,
        height,
        pixels,
    })
}

/// Convert a mask (`0x00` = draw, non-zero = leave alone) into the
/// game-facing black-mask sprite: black where the mask is clear and
/// transparent (`0xff`) where it is set.
fn black_from_mask(mask: &Sprite) -> Sprite {
    Sprite {
        width: mask.width,
        height: mask.height,
        pixels: mask
            .pixels
            .iter()
            .map(|&p| if p == 0 { 0x00 } else { 0xff })
            .collect(),
    }
}

/// Decode the 40 24x24 glyphs from `vgatext.spr`.
///
/// `mkga()` reads the top-left 24x24 pixels of each 32x32 record; the pixel
/// values are palette indices (no transparency).
fn decode_text(data: &[u8]) -> Result<Vec<Sprite>, String> {
    let expected = TEXT_GLYPH_COUNT * SPRITE_RECORD_SIZE;
    if data.len() != expected {
        return Err(format!(
            "vgatext.spr: expected {expected} bytes, got {}",
            data.len()
        ));
    }

    let mut glyphs = Vec::with_capacity(TEXT_GLYPH_COUNT);
    for i in 0..TEXT_GLYPH_COUNT {
        let record = &data[i * SPRITE_RECORD_SIZE..(i + 1) * SPRITE_RECORD_SIZE];
        let mut pixels = Vec::with_capacity(24 * 24);
        for y in 0..24 {
            for x in 0..24 {
                pixels.push(record[y * RECORD_ROW_BYTES + x]);
            }
        }
        glyphs.push(Sprite {
            width: 24,
            height: 24,
            pixels,
        });
    }
    Ok(glyphs)
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, String> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| format!("BMP truncated: need 2 bytes at offset {offset}"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, String> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| format!("BMP truncated: need 4 bytes at offset {offset}"))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_i32(data: &[u8], offset: usize) -> Result<i32, String> {
    Ok(read_u32(data, offset)? as i32)
}

/// Decode the 4bpp BI_RLE4 `vtitle.bmp`.
///
/// The RLE stream is stored bottom-up, so the first decoded line is written
/// to the last row of the top-down output buffer.
///
/// The encoded-run handling follows Microsoft's RLE4 in the same way as
/// FFmpeg's `msrle_decode_pal4()`: a run stores one value byte and repeats
/// its high nibble, low nibble, high nibble, ... for `count` pixels.
/// Absolute runs store `ceil(count/2)` bytes (high nibble first) and are
/// word padded. `0x00 0x00` is end-of-line, `0x00 0x01` end-of-bitmap and
/// `0x00 0x02 dx dy` is a delta.
fn decode_title(data: &[u8]) -> Result<(Sprite, [Rgb; 16]), String> {
    if data.len() < 14 {
        return Err("vtitle.bmp: file too short for a BMP header".to_string());
    }
    if &data[0..2] != b"BM" {
        return Err("vtitle.bmp: missing 'BM' signature".to_string());
    }

    let off_bits = read_u32(data, 10)? as usize;
    let dib_size = read_u32(data, 14)? as usize;
    if dib_size < 40 {
        return Err(format!(
            "vtitle.bmp: unsupported DIB header size {dib_size} (need >= 40)"
        ));
    }
    if 14usize.saturating_add(dib_size) > data.len() {
        return Err("vtitle.bmp: DIB header extends past end of file".to_string());
    }

    let width_i = read_i32(data, 18)?;
    let height_i = read_i32(data, 22)?;
    let planes = read_u16(data, 26)?;
    let bpp = read_u16(data, 28)?;
    let compression = read_u32(data, 30)?;
    let size_image = read_u32(data, 34)? as usize;

    if width_i <= 0 || height_i <= 0 {
        return Err(format!(
            "vtitle.bmp: invalid dimensions {width_i}x{height_i}"
        ));
    }
    // The title is a fixed full-screen asset; rejecting other sizes also
    // keeps the allocation below bounded for malformed input.
    if width_i != 640 || height_i != 400 {
        return Err(format!(
            "vtitle.bmp: expected 640x400, got {width_i}x{height_i}"
        ));
    }
    if planes != 1 {
        return Err(format!("vtitle.bmp: expected 1 plane, got {planes}"));
    }
    if bpp != 4 {
        return Err(format!("vtitle.bmp: expected 4 bpp, got {bpp}"));
    }
    if compression != 2 {
        return Err(format!(
            "vtitle.bmp: expected BI_RLE4 (compression 2), got {compression}"
        ));
    }

    // 16-entry palette immediately after the DIB header, stored B,G,R,0.
    let palette_offset = 14usize.saturating_add(dib_size);
    if palette_offset.saturating_add(16 * 4) > data.len() {
        return Err("vtitle.bmp: truncated colour table".to_string());
    }
    let mut title_palette = [[0u8; 3]; 16];
    for (i, entry) in title_palette.iter_mut().enumerate() {
        let base = palette_offset + i * 4;
        entry[0] = data[base + 2]; // red
        entry[1] = data[base + 1]; // green
        entry[2] = data[base]; // blue
    }

    if off_bits >= data.len() {
        return Err(format!(
            "vtitle.bmp: pixel data offset {off_bits} is past end of file"
        ));
    }
    let end = off_bits
        .checked_add(size_image)
        .filter(|&end| end <= data.len())
        .unwrap_or(data.len());
    let rle = &data[off_bits..end];

    let width = width_i as usize;
    let height = height_i as usize;
    let mut pixels = vec![0u8; width * height];
    let mut line: isize = height as isize - 1;
    let mut x: usize = 0;
    let mut pos = 0usize;
    let mut finished = false;

    while pos < rle.len() && line >= 0 {
        let code = rle[pos];
        pos += 1;

        if code == 0 {
            let command = *rle.get(pos).ok_or_else(|| {
                "vtitle.bmp: truncated RLE escape at end of pixel data".to_string()
            })?;
            pos += 1;

            match command {
                0 => {
                    // End of line.
                    x = 0;
                    line -= 1;
                    if line < 0 {
                        finished = true;
                    }
                }
                1 => {
                    // End of bitmap.
                    finished = true;
                    break;
                }
                2 => {
                    // Delta: skip dx pixels and dy lines.
                    if pos + 2 > rle.len() {
                        return Err("vtitle.bmp: truncated RLE delta".to_string());
                    }
                    let dx = rle[pos] as usize;
                    let dy = rle[pos + 1] as isize;
                    pos += 2;
                    x += dx;
                    line -= dy;
                    if line < 0 || x >= width {
                        return Err(format!(
                            "vtitle.bmp: RLE delta out of bounds (x={x}, line={line})"
                        ));
                    }
                }
                count => {
                    // Absolute run: `count` pixels, high nibble first.
                    let count = count as usize;
                    let byte_count = (count + 1) / 2;
                    let odd = count & 1 != 0;
                    if pos + byte_count > rle.len() {
                        return Err("vtitle.bmp: truncated absolute RLE run".to_string());
                    }
                    for k in 0..byte_count {
                        let byte = rle[pos + k];
                        if x >= width {
                            break;
                        }
                        pixels[line as usize * width + x] = byte >> 4;
                        x += 1;
                        if k + 1 == byte_count && odd {
                            break;
                        }
                        if x >= width {
                            break;
                        }
                        pixels[line as usize * width + x] = byte & 0x0f;
                        x += 1;
                    }
                    pos += byte_count;
                    // Absolute runs are padded to a 16-bit boundary.
                    if byte_count & 1 != 0 {
                        pos += 1;
                    }
                }
            }
        } else {
            // Encoded run: one value byte whose nibbles alternate.
            let value = *rle
                .get(pos)
                .ok_or_else(|| "vtitle.bmp: truncated encoded RLE run".to_string())?;
            pos += 1;
            let run = code as usize;
            if x + run > width + 1 {
                return Err(format!(
                    "vtitle.bmp: encoded run of {run} overflows row at x={x}"
                ));
            }
            for k in 0..run {
                if x >= width {
                    break;
                }
                let value = if k & 1 == 0 { value >> 4 } else { value & 0x0f };
                pixels[line as usize * width + x] = value;
                x += 1;
            }
        }
    }

    if !finished {
        return Err("vtitle.bmp: RLE data ended before the bitmap was complete".to_string());
    }

    Ok((
        Sprite {
            width,
            height,
            pixels,
        },
        title_palette,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assets() -> Assets {
        Assets::load().expect("embedded assets should decode")
    }

    #[test]
    fn raw_record_layout() {
        assert_eq!(VGA_SPR.len(), RAW_SPRITE_COUNT * SPRITE_RECORD_SIZE);
        assert_eq!(VGA_SPR.len(), 107 * 1024);
        assert_eq!(WIDTH_UNITS.len(), RAW_SPRITE_COUNT);
        assert_eq!(HEIGHT_UNITS.len(), RAW_SPRITE_COUNT);
    }

    #[test]
    fn sprite_count_and_dimensions() {
        let assets = assets();
        assert_eq!(assets.sprites.len(), RAW_SPRITE_COUNT);
        for (i, sprite) in assets.sprites.iter().enumerate() {
            assert_eq!(
                sprite.width,
                WIDTH_UNITS[i] * 8,
                "sprite {i} width mismatch"
            );
            assert_eq!(
                sprite.height,
                HEIGHT_UNITS[i] * 2,
                "sprite {i} height mismatch"
            );
            assert_eq!(
                sprite.pixels.len(),
                sprite.width * sprite.height,
                "sprite {i} pixel count mismatch"
            );
        }
    }

    #[test]
    fn background_tiles_are_40x8_and_opaque() {
        let assets = assets();
        for i in 87..=94 {
            let sprite = &assets.sprites[i];
            assert_eq!(sprite.width, 40, "background {i}");
            assert_eq!(sprite.height, 8, "background {i}");
            // Background tiles have no 0xff transparency marker.
            assert!(
                sprite.pixels.iter().all(|&p| p != 0xff),
                "background {i} must be opaque"
            );
        }
    }

    #[test]
    fn special_raw_sprites_have_expected_shapes() {
        let assets = assets();
        // rightblob (80) is the tall vertical rounded-edge mask.
        assert_eq!(
            (assets.sprites[80].width, assets.sprites[80].height),
            (16, 36)
        );
        // leftblob (81) has the same tall layout.
        assert_eq!(
            (assets.sprites[81].width, assets.sprites[81].height),
            (16, 36)
        );
        // topblob / bottomblob / squareblob are wide, furryblob is taller.
        assert_eq!(
            (assets.sprites[82].width, assets.sprites[82].height),
            (48, 12)
        );
        assert_eq!(
            (assets.sprites[83].width, assets.sprites[83].height),
            (48, 12)
        );
        assert_eq!(
            (assets.sprites[84].width, assets.sprites[84].height),
            (48, 16)
        );
        assert_eq!(
            (assets.sprites[85].width, assets.sprites[85].height),
            (48, 12)
        );
        // The mask records only contain 0x00 / 0x0f.
        for i in [78usize, 80, 81, 82, 83, 84, 85, 86] {
            assert!(
                assets.sprites[i]
                    .pixels
                    .iter()
                    .all(|&p| p == 0 || p == 0x0f),
                "raw {i} should be a 0/0x0f mask"
            );
        }
        // A vertical edge mask is not empty.
        assert!(assets.sprites[80].pixels.iter().any(|&p| p != 0));
        assert!(assets.sprites[82].pixels.iter().any(|&p| p != 0));
    }

    #[test]
    fn text_has_40_glyphs() {
        let assets = assets();
        assert_eq!(assets.text.len(), TEXT_GLYPH_COUNT);
        let a = assets.glyph('A').expect("'A' glyph");
        assert_eq!((a.width, a.height), (24, 24));
        assert_eq!(a.pixels.len(), 24 * 24);
        assert!(a.pixels.iter().any(|&p| p != 0), "'A' should not be blank");
    }

    #[test]
    fn glyph_lookup_is_case_insensitive_and_bounded() {
        let assets = assets();
        assert_eq!(assets.glyph('a'), assets.glyph('A'));
        assert_eq!(assets.glyph('z'), assets.glyph('Z'));
        assert!(assets.glyph('0').is_some());
        assert!(assets.glyph('.').is_some());
        assert!(assets.glyph(':').is_some());
        assert!(assets.glyph('_').is_some());
        assert!(assets.glyph(' ').is_some());
        assert!(assets.glyph('!').is_none());
        assert!(assets.glyph('é').is_none());
    }

    #[test]
    fn title_is_640x400_with_content() {
        let assets = assets();
        assert_eq!((assets.title.width, assets.title.height), (640, 400));
        assert_eq!(assets.title.pixels.len(), 640 * 400);
        assert!(
            assets.title.pixels.iter().any(|&p| p != 0),
            "title should contain visible pixels"
        );
        // 4bpp title uses palette indices 0..=15.
        assert!(assets.title.pixels.iter().all(|&p| p < 16));
        // The title palette is the BMP's own table (B,G,R -> R,G,B).
        assert_eq!(assets.title_palette[0], [0, 0, 0]);
        assert_eq!(assets.title_palette[1], [0, 0, 128]);
        assert_eq!(assets.title_palette[4], [128, 0, 0]);
    }

    #[test]
    fn title_decodes_exactly() {
        // Regression check: this FNV-1a checksum was produced by an
        // independent RLE4 decode of `vtitle.bmp` (validated pixel-for-pixel
        // against FFmpeg and macOS `sips`). It catches subtle mistakes in
        // encoded runs, absolute runs, padding, delta handling and row
        // orientation.
        let assets = assets();
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for &p in &assets.title.pixels {
            hash ^= p as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        assert_eq!(hash, 0x5a74_f9e9_b8ec_1dd8);
        assert_eq!(
            assets.title.pixels.iter().filter(|&&p| p != 0).count(),
            17_938
        );
    }

    #[test]
    fn every_game_index_resolves() {
        let assets = assets();
        for ch in 0..GAME_SPRITE_COUNT {
            assert!(
                game_sprite_raw_index(ch).is_some(),
                "game sprite {ch} missing from table"
            );
            assert!(
                assets.game_sprite_ref(ch).is_some(),
                "game sprite {ch} missing"
            );
        }
        assert!(game_sprite_raw_index(GAME_SPRITE_COUNT).is_none());
        assert!(game_sprite_raw_index(usize::MAX).is_none());
    }

    #[test]
    fn known_game_sprite_mappings() {
        assert_eq!(game_sprite_raw_index(1), Some(GameSprite::Raw(25))); // rdigger1
        assert_eq!(game_sprite_raw_index(69), Some(GameSprite::Raw(10))); // nobbin1
        assert_eq!(game_sprite_raw_index(81), Some(GameSprite::Raw(20))); // bonus
        assert_eq!(game_sprite_raw_index(94), Some(GameSprite::Raw(87))); // back1
        assert_eq!(game_sprite_raw_index(102), Some(GameSprite::BlackMask(80))); // rightblob
        assert_eq!(game_sprite_raw_index(108), Some(GameSprite::Raw(0))); // emerald
        assert_eq!(game_sprite_raw_index(114), Some(GameSprite::Raw(101))); // fire1b

        // Special cases from the vgatable zero-data entries.
        assert_eq!(game_sprite_raw_index(0), Some(GameSprite::Blank));
        assert_eq!(game_sprite_raw_index(31), Some(GameSprite::Blank));
        assert_eq!(game_sprite_raw_index(109), Some(GameSprite::EraseEmerald));
    }

    #[test]
    fn sprite_resolution_special_cases() {
        let assets = assets();

        assert!(assets.sprite(0).is_none());
        assert!(assets.sprite(31).is_none());

        // EraseEmerald hands back the emerald artwork.
        let emerald = assets.sprite(108).expect("emerald");
        assert_eq!((emerald.width, emerald.height), (32, 20));
        let erase = assets.sprite(109).expect("erase emerald");
        assert_eq!(erase.width, emerald.width);
        assert_eq!(erase.height, emerald.height);
        assert_eq!(erase.pixels, emerald.pixels);

        // Black masks are black/transparent only.
        let blob = assets.sprite(102).expect("rightblob");
        assert_eq!((blob.width, blob.height), (16, 36));
        assert!(blob.pixels.iter().all(|&p| p == 0x00 || p == 0xff));
        assert!(blob.pixels.iter().any(|&p| p == 0x00));
        assert!(blob.pixels.iter().any(|&p| p == 0xff));

        // "elife" (game 112) is used with zero data and an all-clear normal
        // mask, so it paints a solid black rectangle.
        let elife = assets.sprite(112).expect("elife erase");
        assert_eq!((elife.width, elife.height), (32, 24));
        assert!(elife.pixels.iter().all(|&p| p == 0x00));
    }
}
