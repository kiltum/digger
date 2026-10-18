//! Port of `sprite.c`'s entity records and collision bookkeeping.
//!
//! The original keeps a save/restore buffer for every moving sprite so that it
//! can erase a sprite and repaint whatever was behind it.  Our renderer instead
//! keeps a dirt/background layer and redraws every enabled sprite from these
//! records on each frame, so only the *state* half of `sprite.c` is needed here.
//!
//! The collision bookkeeping is kept bit-for-bit compatible with the original:
//! `drawspr()` recomputes `first`/`coll` for the sprite that just moved, and the
//! game modules copy those arrays immediately afterwards.

use crate::assets::Assets;
use crate::screen::Screen;

pub const TYPES: usize = 5;
pub const BONUSES: usize = 1;
pub const BAGS: usize = 50;
pub const MONSTERS: usize = 6;
pub const FIREBALLS: usize = 2;
pub const DIGGERS: usize = 2;
pub const SPRITES: usize = BONUSES + BAGS + MONSTERS + FIREBALLS + DIGGERS;

pub const FIRSTBONUS: usize = 0;
pub const LASTBONUS: usize = FIRSTBONUS + BONUSES;
pub const FIRSTBAG: usize = LASTBONUS;
pub const LASTBAG: usize = FIRSTBAG + BAGS;
pub const FIRSTMONSTER: usize = LASTBAG;
pub const LASTMONSTER: usize = FIRSTMONSTER + MONSTERS;
pub const FIRSTFIREBALL: usize = LASTMONSTER;
pub const LASTFIREBALL: usize = FIRSTFIREBALL + FIREBALLS;
pub const FIRSTDIGGER: usize = LASTFIREBALL;
pub const LASTDIGGER: usize = FIRSTDIGGER + DIGGERS;

pub const FIRSTT: [usize; TYPES] = [
    FIRSTBONUS,
    FIRSTBAG,
    FIRSTMONSTER,
    FIRSTFIREBALL,
    FIRSTDIGGER,
];
pub const LASTT: [usize; TYPES] = [LASTBONUS, LASTBAG, LASTMONSTER, LASTFIREBALL, LASTDIGGER];

#[derive(Clone, Copy, Debug, Default)]
pub struct SpriteRec {
    /// `sprenf[]`
    pub enabled: bool,
    /// `sprch[]` — current game sprite number.
    pub ch: i32,
    /// `sprx[]`, in 320x200 logical coordinates (always 4-pixel aligned).
    pub x: i32,
    /// `spry[]`
    pub y: i32,
    /// `sprwid[]`, `sprhei[]` — logical size of the sprite that is drawn.
    pub w: i32,
    pub h: i32,
    /// `sprbwid[]`, `sprbhei[]` — size of the opaque part, used for collision.
    pub bw: i32,
    pub bh: i32,
    /// `sprnch[]`, `sprnwid[]`, ... — "next" sprite, selected by `initspr()`.
    pub nch: i32,
    pub nw: i32,
    pub nh: i32,
    pub nbw: i32,
    pub nbh: i32,
}

#[derive(Clone)]
pub struct SpriteSystem {
    pub recs: [SpriteRec; SPRITES],
    /// `first[TYPES]` from `bcollides()`.
    pub first: [i32; TYPES],
    /// `coll[SPRITES]` from `bcollides()`.
    pub coll: [i32; SPRITES],
}

impl Default for SpriteSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl SpriteSystem {
    pub fn new() -> Self {
        Self {
            recs: [SpriteRec::default(); SPRITES],
            first: [-1; TYPES],
            coll: [-1; SPRITES],
        }
    }

    /// `createspr()` / `initmbspr()` reset.
    pub fn createspr(&mut self, n: usize, ch: i32, w: i32, h: i32, bw: i32, bh: i32) {
        let r = &mut self.recs[n];
        r.nch = ch;
        r.ch = ch;
        r.nw = w;
        r.w = w;
        r.nh = h;
        r.h = h;
        r.nbw = bw;
        r.bw = bw;
        r.nbh = bh;
        r.bh = bh;
        r.enabled = false;
    }

    /// `initspr()` — select the sprite for the next draw.
    pub fn initspr(&mut self, n: usize, ch: i32, w: i32, h: i32, bw: i32, bh: i32) {
        let r = &mut self.recs[n];
        r.nch = ch;
        r.nw = w;
        r.nh = h;
        r.nbw = bw;
        r.nbh = bh;
    }

    /// `movedrawspr()` — place a sprite without recomputing collisions.
    pub fn movedrawspr(&mut self, n: usize, x: i32, y: i32) {
        let r = &mut self.recs[n];
        r.x = x & !3;
        r.y = y;
        r.ch = r.nch;
        r.w = r.nw;
        r.h = r.nh;
        r.bw = r.nbw;
        r.bh = r.nbh;
        r.enabled = true;
    }

    /// `erasespr()` — the pixel restore is implicit in our compositor.
    pub fn erasespr(&mut self, n: usize) {
        self.recs[n].enabled = false;
    }

    /// `drawspr()` — place/animate a sprite and recompute `first`/`coll`.
    pub fn drawspr(&mut self, n: usize, x: i32, y: i32) {
        let x = x & !3;
        {
            let r = &mut self.recs[n];
            r.enabled = true;
            r.x = x;
            r.y = y;
            r.ch = r.nch;
            r.w = r.nw;
            r.h = r.nh;
            r.bw = r.nbw;
            r.bh = r.nbh;
        }
        self.bcollides(n);
    }

    /// `collide()` from `sprite.c`.
    pub fn collide(&self, bx: usize, si: usize) -> bool {
        let a = &self.recs[bx];
        let b = &self.recs[si];
        if a.x >= b.x {
            if a.x > (b.w << 2) + b.x - 1 {
                return false;
            }
        } else if b.x > (a.w << 2) + a.x - 1 {
            return false;
        }
        if a.y >= b.y {
            return a.y <= b.h + b.y - 1;
        }
        b.y <= a.h + a.y - 1
    }

    /// `bcollide()` from `sprite.c` (uses the opaque bounding box).
    pub fn bcollide(&self, bx: usize, si: usize) -> bool {
        let a = &self.recs[bx];
        let b = &self.recs[si];
        if a.x >= b.x {
            if a.x + a.bw > (b.w << 2) + b.x - b.bw - 1 {
                return false;
            }
        } else if b.x + b.bw > (a.w << 2) + a.x - a.bw - 1 {
            return false;
        }
        if a.y >= b.y {
            return a.y + a.bh <= b.h + b.y - b.bh - 1;
        }
        b.y + b.bh <= a.h + a.y - a.bh - 1
    }

    /// `bcollides()` — build the collision lists for sprite `spr`.
    pub fn bcollides(&mut self, spr: usize) {
        self.first = [-1; TYPES];
        self.coll = [-1; SPRITES];
        for t in 0..TYPES {
            let mut next: i32 = -1;
            for spc in FIRSTT[t]..LASTT[t] {
                if self.recs[spc].enabled && spc != spr && self.bcollide(spr, spc) {
                    if next == -1 {
                        self.first[t] = spc as i32;
                        next = spc as i32;
                    } else {
                        self.coll[next as usize] = spc as i32;
                        next = spc as i32;
                        self.coll[next as usize] = -1;
                    }
                }
            }
        }
    }

    /// Draw every enabled entity over `screen`.
    pub fn compose(&self, screen: &mut Screen, assets: &Assets) {
        for r in self.recs.iter() {
            if r.enabled {
                screen.blit_game(assets, r.x, r.y, r.ch as usize, r.w, r.h);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_boxes_collide() {
        let mut s = SpriteSystem::new();
        s.createspr(0, 1, 4, 15, 0, 0);
        s.createspr(1, 1, 4, 15, 0, 0);
        s.movedrawspr(0, 100, 100);
        s.movedrawspr(1, 104, 100);
        assert!(s.bcollide(0, 1));
        s.movedrawspr(1, 116, 100);
        assert!(!s.bcollide(0, 1));
    }

    #[test]
    fn bcollides_builds_linked_lists() {
        let mut s = SpriteSystem::new();
        for i in 0..SPRITES {
            s.createspr(i, 1, 4, 15, 0, 0);
        }
        s.movedrawspr(FIRSTDIGGER, 100, 100);
        s.movedrawspr(FIRSTMONSTER, 104, 100);
        s.movedrawspr(FIRSTMONSTER + 1, 108, 100);
        s.drawspr(FIRSTDIGGER, 100, 100);
        // first[2] is the first monster; its coll link points to the second.
        assert_eq!(s.first[2], FIRSTMONSTER as i32);
        assert_eq!(s.coll[FIRSTMONSTER], (FIRSTMONSTER + 1) as i32);
        assert_eq!(s.coll[FIRSTMONSTER + 1], -1);
    }
}
