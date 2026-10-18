//! Port of `bags.c`: emerald bags, wobbling, falling, gold and pushing.

use crate::constants::*;
use crate::game::Game;
use crate::sprites::*;

const WBLANIM: [i32; 4] = [2, 0, 1, 0];

impl Game {
    pub fn initbags(&mut self) {
        self.pushcount = 0;
        self.goldtime = 150 - self.levof10() * 10;
        for b in self.bagdat.iter_mut() {
            b.exist = false;
        }
        let mut bag = 0usize;
        for x in 0..MWIDTH {
            for y in 0..MHEIGHT {
                if self.getlevch(x as i32, y as i32, self.levplan()) == b'B' && bag < BAGS {
                    let b = &mut self.bagdat[bag];
                    b.exist = true;
                    b.gt = 0;
                    b.fallh = 0;
                    b.dir = DIR_NONE;
                    b.wobbling = false;
                    b.wt = 15;
                    b.unfallen = true;
                    b.x = x as i32 * 20 + 12;
                    b.y = y as i32 * 18 + 18;
                    b.h = x as i32;
                    b.v = y as i32;
                    b.xr = 0;
                    b.yr = 0;
                    bag += 1;
                }
            }
        }
        if self.curplayer == 0 {
            self.bagdat1 = self.bagdat;
        } else {
            self.bagdat2 = self.bagdat;
        }
    }

    pub fn drawbags(&mut self) {
        if self.curplayer == 0 {
            self.bagdat = self.bagdat1;
        } else {
            self.bagdat = self.bagdat2;
        }
        for bag in 0..BAGS {
            if self.bagdat[bag].exist {
                let (x, y) = (self.bagdat[bag].x, self.bagdat[bag].y);
                self.sys.sprites.movedrawspr(bag + FIRSTBAG, x, y);
            }
        }
    }

    pub fn cleanupbags(&mut self) {
        self.sys.sound_fall_off();
        for bag in 0..BAGS {
            let b = &self.bagdat[bag];
            if b.exist
                && ((b.h == 7 && b.v == 9)
                    || b.xr != 0
                    || b.yr != 0
                    || b.gt != 0
                    || b.fallh != 0
                    || b.wobbling)
            {
                self.bagdat[bag].exist = false;
                self.sys.sprites.erasespr(bag + FIRSTBAG);
            }
        }
        if self.curplayer == 0 {
            self.bagdat1 = self.bagdat;
        } else {
            self.bagdat2 = self.bagdat;
        }
    }

    pub fn dobags(&mut self) {
        let mut soundfalloffflag = true;
        let mut soundwobbleoffflag = true;
        for bag in 0..BAGS {
            if !self.bagdat[bag].exist {
                continue;
            }
            if self.bagdat[bag].gt != 0 {
                if self.bagdat[bag].gt == 1 {
                    self.sys.sound_break();
                    let (x, y) = (self.bagdat[bag].x, self.bagdat[bag].y);
                    self.drawgold(bag, 4, x, y);
                    self.incpenalty();
                }
                if self.bagdat[bag].gt == 3 {
                    let (x, y) = (self.bagdat[bag].x, self.bagdat[bag].y);
                    self.drawgold(bag, 5, x, y);
                    self.incpenalty();
                }
                if self.bagdat[bag].gt == 5 {
                    let (x, y) = (self.bagdat[bag].x, self.bagdat[bag].y);
                    self.drawgold(bag, 6, x, y);
                    self.incpenalty();
                }
                self.bagdat[bag].gt += 1;
                if self.bagdat[bag].gt == self.goldtime {
                    self.removebag(bag);
                } else if self.bagdat[bag].v < MHEIGHT as i32 - 1
                    && self.bagdat[bag].gt < self.goldtime - 10
                {
                    let (h, v) = (self.bagdat[bag].h, self.bagdat[bag].v);
                    if self.getfield(h, v + 1) & 0x2000 == 0 {
                        self.bagdat[bag].gt = self.goldtime - 10;
                    }
                }
            } else {
                self.updatebag(bag);
            }
        }
        for bag in 0..BAGS {
            if self.bagdat[bag].dir == DIR_DOWN && self.bagdat[bag].exist {
                soundfalloffflag = false;
            }
            if self.bagdat[bag].dir != DIR_DOWN
                && self.bagdat[bag].wobbling
                && self.bagdat[bag].exist
            {
                soundwobbleoffflag = false;
            }
        }
        if soundfalloffflag {
            self.sys.sound_fall_off();
        }
        if soundwobbleoffflag {
            self.sys.sound_wobble_off();
        }
    }

    fn updatebag(&mut self, bag: usize) {
        let x = self.bagdat[bag].x;
        let h = self.bagdat[bag].h;
        let xr = self.bagdat[bag].xr;
        let y = self.bagdat[bag].y;
        let v = self.bagdat[bag].v;
        let yr = self.bagdat[bag].yr;
        match self.bagdat[bag].dir {
            DIR_NONE => 'dir_none: {
                if y < 180 && xr == 0 {
                    if self.bagdat[bag].wobbling {
                        if self.bagdat[bag].wt == 0 {
                            self.bagdat[bag].dir = DIR_DOWN;
                            self.sys.sound_fall();
                            // The original does `break` out of the switch here,
                            // then falls through to the push code below.
                            break 'dir_none;
                        }
                        self.bagdat[bag].wt -= 1;
                        let wbl = self.bagdat[bag].wt % 8;
                        if wbl & 1 == 0 {
                            self.drawgold(bag, WBLANIM[(wbl >> 1) as usize], x, y);
                            self.incpenalty();
                            self.sys.sound_wobble();
                        }
                    } else if self.getfield(h, v + 1) & 0x0fdf != 0x0fdf
                        && !self.checkdiggerunderbag(h, v + 1)
                    {
                        self.bagdat[bag].wobbling = true;
                    }
                } else {
                    self.bagdat[bag].wt = 15;
                    self.bagdat[bag].wobbling = false;
                }
            }
            DIR_RIGHT | DIR_LEFT => {
                if xr == 0 {
                    if y < 180 && self.getfield(h, v + 1) & 0x0fdf != 0x0fdf {
                        self.bagdat[bag].dir = DIR_DOWN;
                        self.bagdat[bag].wt = 0;
                        self.sys.sound_fall();
                    } else {
                        self.baghitground(bag);
                    }
                }
            }
            DIR_DOWN => {
                if yr == 0 {
                    self.bagdat[bag].fallh += 1;
                }
                if y >= 180 {
                    self.baghitground(bag);
                } else if self.getfield(h, v + 1) & 0x0fdf == 0x0fdf {
                    if yr == 0 {
                        self.baghitground(bag);
                    }
                }
                self.checkmonscared(self.bagdat[bag].h);
            }
            _ => {}
        }
        if self.bagdat[bag].dir != DIR_NONE {
            if self.bagdat[bag].dir != DIR_DOWN && self.pushcount != 0 {
                self.pushcount -= 1;
            } else {
                let dir = self.bagdat[bag].dir;
                self.pushbag(bag, dir);
            }
        }
    }

    fn baghitground(&mut self, bag: usize) {
        if self.bagdat[bag].dir == DIR_DOWN && self.bagdat[bag].fallh > 1 {
            self.bagdat[bag].gt = 1;
        } else {
            self.bagdat[bag].fallh = 0;
        }
        self.bagdat[bag].dir = DIR_NONE;
        self.bagdat[bag].wt = 15;
        self.bagdat[bag].wobbling = false;
        let (x, y) = (self.bagdat[bag].x, self.bagdat[bag].y);
        self.drawgold(bag, 0, x, y);
        let clfirst = self.clfirst();
        let clcoll = self.clcoll();
        self.incpenalty();
        let mut i = clfirst[1];
        while i != -1 {
            self.removebag((i - FIRSTBAG as i32) as usize);
            i = clcoll[i as usize];
        }
    }

    fn pushbag(&mut self, bag: usize, dir: i32) -> bool {
        let mut x = self.bagdat[bag].x;
        let mut y = self.bagdat[bag].y;
        let ox = x;
        let oy = y;
        let h = self.bagdat[bag].h;
        let v = self.bagdat[bag].v;
        let mut push = true;

        if self.bagdat[bag].gt != 0 {
            self.getgold(bag);
            return true;
        }
        if self.bagdat[bag].dir == DIR_DOWN && (dir == DIR_RIGHT || dir == DIR_LEFT) {
            self.drawgold(bag, 3, x, y);
            let clfirst = self.clfirst();
            let clcoll = self.clcoll();
            self.incpenalty();
            let mut i = clfirst[4];
            while i != -1 {
                let d = (i - FIRSTDIGGER as i32 + self.curplayer) as usize;
                if self.diggery(d) >= y {
                    self.killdigger(d, 1, bag as i32);
                }
                i = clcoll[i as usize];
            }
            if clfirst[2] != -1 {
                self.squashmonsters(bag as i32, &clfirst, &clcoll);
            }
            return true;
        }
        if (x == 292 && dir == DIR_RIGHT)
            || (x == 12 && dir == DIR_LEFT)
            || (y == 180 && dir == DIR_DOWN)
            || (y == 18 && dir == DIR_UP)
        {
            push = false;
        }
        if push {
            match dir {
                DIR_RIGHT => x += 4,
                DIR_LEFT => x -= 4,
                DIR_DOWN => {
                    if self.bagdat[bag].unfallen {
                        self.bagdat[bag].unfallen = false;
                        self.sys.screen.draw_square_blob(x, y);
                        self.sys.screen.draw_top_blob(x, y + 21);
                    } else {
                        self.sys.screen.draw_furry_blob(x, y);
                    }
                    self.eatfield(x, y, dir);
                    self.killemerald(h, v);
                    y += 6;
                }
                _ => {}
            }
            match dir {
                DIR_DOWN => {
                    self.drawgold(bag, 3, x, y);
                    let clfirst = self.clfirst();
                    let clcoll = self.clcoll();
                    self.incpenalty();
                    let mut i = clfirst[4];
                    while i != -1 {
                        let d = (i - FIRSTDIGGER as i32 + self.curplayer) as usize;
                        if self.diggery(d) >= y {
                            self.killdigger(d, 1, bag as i32);
                        }
                        i = clcoll[i as usize];
                    }
                    if clfirst[2] != -1 {
                        self.squashmonsters(bag as i32, &clfirst, &clcoll);
                    }
                }
                DIR_RIGHT | DIR_LEFT => {
                    self.bagdat[bag].wt = 15;
                    self.bagdat[bag].wobbling = false;
                    self.drawgold(bag, 0, x, y);
                    let clfirst = self.clfirst();
                    let clcoll = self.clcoll();
                    self.incpenalty();
                    self.pushcount = 1;
                    if clfirst[1] != -1 && !self.pushbags(dir, &clfirst, &clcoll) {
                        x = ox;
                        y = oy;
                        self.drawgold(bag, 0, ox, oy);
                        self.incpenalty();
                        push = false;
                    }
                    let mut i = clfirst[4];
                    let mut digf = false;
                    while i != -1 {
                        let d = (i - FIRSTDIGGER as i32 + self.curplayer) as usize;
                        if self.digalive(d as i32) {
                            digf = true;
                        }
                        i = clcoll[i as usize];
                    }
                    if digf || clfirst[2] != -1 {
                        x = ox;
                        y = oy;
                        self.drawgold(bag, 0, ox, oy);
                        self.incpenalty();
                        push = false;
                    }
                }
                _ => {}
            }
            if push {
                self.bagdat[bag].dir = dir;
            } else {
                self.bagdat[bag].dir = self.reversedir(dir);
            }
            self.bagdat[bag].x = x;
            self.bagdat[bag].y = y;
            self.bagdat[bag].h = (x - 12) / 20;
            self.bagdat[bag].v = (y - 18) / 18;
            self.bagdat[bag].xr = (x - 12) % 20;
            self.bagdat[bag].yr = (y - 18) % 18;
        }
        push
    }

    pub fn pushbags(&mut self, dir: i32, clfirst: &[i32; TYPES], clcoll: &[i32; SPRITES]) -> bool {
        let mut push = true;
        let mut next = clfirst[1];
        while next != -1 {
            if !self.pushbag((next - FIRSTBAG as i32) as usize, dir) {
                push = false;
            }
            next = clcoll[next as usize];
        }
        push
    }

    pub fn pushudbags(&mut self, clfirst: &[i32; TYPES], clcoll: &[i32; SPRITES]) -> bool {
        let mut push = true;
        let mut next = clfirst[1];
        while next != -1 {
            let b = (next - FIRSTBAG as i32) as usize;
            if self.bagdat[b].gt != 0 {
                self.getgold(b);
            } else {
                push = false;
            }
            next = clcoll[next as usize];
        }
        push
    }

    pub fn removebag(&mut self, bag: usize) {
        if self.bagdat[bag].exist {
            self.bagdat[bag].exist = false;
            self.sys.sprites.erasespr(bag + FIRSTBAG);
        }
    }

    pub fn bagexist(&self, bag: i32) -> bool {
        if bag < 0 || bag >= BAGS as i32 {
            return false;
        }
        self.bagdat[bag as usize].exist
    }

    pub fn bagy(&self, bag: i32) -> i32 {
        self.bagdat[bag as usize].y
    }

    pub fn getbagdir(&self, bag: i32) -> i32 {
        if self.bagdat[bag as usize].exist {
            self.bagdat[bag as usize].dir
        } else {
            -1
        }
    }

    pub fn removebags(&mut self, clfirst: &[i32; TYPES], clcoll: &[i32; SPRITES]) {
        let mut next = clfirst[1];
        while next != -1 {
            self.removebag((next - FIRSTBAG as i32) as usize);
            next = clcoll[next as usize];
        }
    }

    pub fn getnmovingbags(&self) -> i32 {
        let mut n = 0;
        for b in &self.bagdat {
            if b.exist && b.gt < 10 && (b.gt != 0 || b.wobbling) {
                n += 1;
            }
        }
        n
    }

    fn getgold(&mut self, bag: usize) {
        let mut f = true;
        let (x, y) = (self.bagdat[bag].x, self.bagdat[bag].y);
        self.drawgold(bag, 6, x, y);
        self.incpenalty();
        let clcoll = self.clcoll();
        let mut i = self.sys.sprites.first[4];
        while i != -1 {
            let d = (i - FIRSTDIGGER as i32 + self.curplayer) as usize;
            if self.digalive(d as i32) {
                self.scoregold(d);
                self.sys.sound_gold();
                self.digresettime(d);
                f = false;
            }
            i = clcoll[i as usize];
        }
        if f {
            self.mongold();
        }
        self.removebag(bag);
    }
}
