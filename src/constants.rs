//! Shared constants from the original VGA build.

// --- Screen geometry -------------------------------------------------------
/// Logical game resolution (the coordinate space used by all game code).
pub const LOGICAL_W: i32 = 320;
pub const LOGICAL_H: i32 = 200;
/// Physical VGA back-buffer. All art in `vga.spr` is authored at 2x, so game
/// coordinates are multiplied by two when blitting.
pub const FB_W: usize = 640;
pub const FB_H: usize = 400;
pub const SCALE: i32 = 2;

// --- Playfield -------------------------------------------------------------
pub const MWIDTH: usize = 15;
pub const MHEIGHT: usize = 10;
pub const MSIZE: usize = MWIDTH * MHEIGHT;

// --- Directions (same numeric values as the original) ----------------------
pub const DIR_NONE: i32 = -1;
pub const DIR_RIGHT: i32 = 0;
pub const DIR_UP: i32 = 2;
pub const DIR_LEFT: i32 = 4;
pub const DIR_DOWN: i32 = 6;

/// PIT input clock; also the unit of `ftime`, `curtime` and `cgtime`.
pub const PIT_HZ: f64 = 1_193_181.0;
/// Default frame period in PIT ticks (~14.9 fps).
pub const DEFAULT_FTIME: u64 = 80_000;
/// Number of sound-engine updates per second (0x4000 PIT ticks).
pub const SOUND_TICK_HZ: f64 = 1_193_181.0 / 16384.0;

pub const BONUS_SCORE: i64 = 20_000;
pub const START_LIVES: i32 = 3;
