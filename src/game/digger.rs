//! Port of `digger.c`: the player, fireballs, death, bonus mode and emeralds.

use crate::constants::*;
use crate::game::Game;
use crate::sprites::*;

const BITMASKS: [u16; 12] = [
    0xfffe, 0xfffd, 0xfffb, 0xfff7, 0xffef, 0xffdf, 0xffbf, 0xff7f, 0xfeff, 0xfdff, 0xfbff, 0xf7ff,
];
const EMBOX: [i32; 8] = [8, 12, 12, 9, 16, 12, 6, 9];
const DEATHARC: [i32; 7] = [3, 5, 6, 6, 5, 3, 0];

impl Game {
    pub fn makeemfield(&mut self) {
        self.emmask = 1 << self.curplayer;
        for x in 0..MWIDTH {
            for y in 0..MHEIGHT {
                let idx = y * MWIDTH + x;
                if self.getlevch(x as i32, y as i32, self.levplan()) == b'C' {
                    self.emfield[idx] |= self.emmask;
                } else {
                    self.emfield[idx] &= !self.emmask;
                }
            }
        }
    }

    pub fn drawemeralds(&mut self) {
        self.emmask = 1 << self.curplayer;
        for x in 0..MWIDTH {
            for y in 0..MHEIGHT {
                if self.emfield[y * MWIDTH + x] & self.emmask != 0 {
                    self.sys
                        .screen
                        .draw_emerald(x as i32 * 20 + 12, y as i32 * 18 + 21);
                }
            }
        }
    }

    pub fn countem(&self) -> i32 {
        let mut n = 0;
        for &e in &self.emfield {
            if e & self.emmask != 0 {
                n += 1;
            }
        }
        n
    }

    pub fn killemerald(&mut self, x: i32, y: i32) {
        let idx = (y + 1) as usize * MWIDTH + x as usize;
        if idx < MSIZE && self.emfield[idx] & self.emmask != 0 {
            self.emfield[idx] &= !self.emmask;
            self.sys
                .screen
                .erase_emerald(x * 20 + 12, (y + 1) * 18 + 21);
        }
    }

    /// `hitemerald()` — called as the digger moves through a cell.
    pub fn hitemerald(&mut self, x: i32, y: i32, rx: i32, ry: i32, dir: i32) -> bool {
        if dir != DIR_RIGHT && dir != DIR_UP && dir != DIR_LEFT && dir != DIR_DOWN {
            return false;
        }
        let (mut x, mut y) = (x, y);
        if dir == DIR_RIGHT && rx != 0 {
            x += 1;
        }
        if dir == DIR_DOWN && ry != 0 {
            y += 1;
        }
        let r = if dir == DIR_RIGHT || dir == DIR_LEFT {
            rx
        } else {
            ry
        };
        if x < 0 || y < 0 || x >= MWIDTH as i32 || y >= MHEIGHT as i32 {
            return false;
        }
        let idx = y as usize * MWIDTH + x as usize;
        let mut hit = false;
        if self.emfield[idx] & self.emmask != 0 {
            if r == EMBOX[dir as usize] {
                self.sys.screen.draw_emerald(x * 20 + 12, y * 18 + 21);
                self.incpenalty();
            }
            if r == EMBOX[dir as usize + 1] {
                self.sys.screen.erase_emerald(x * 20 + 12, y * 18 + 21);
                self.incpenalty();
                hit = true;
                self.emfield[idx] &= !self.emmask;
            }
        }
        hit
    }

    pub fn initdigger(&mut self) {
        for dig in self.curplayer..self.diggers + self.curplayer {
            let d = &mut self.digdat[dig as usize];
            if d.lives == 0 {
                continue;
            }
            d.v = 9;
            d.mdir = 4;
            d.h = if self.diggers == 1 { 7 } else { 8 - dig * 2 };
            d.x = d.h * 20 + 12;
            d.dir = if dig == 0 { DIR_RIGHT } else { DIR_LEFT };
            d.rx = 0;
            d.ry = 0;
            d.bagtime = 0;
            d.alive = true;
            d.dead = false;
            d.invin = false;
            d.ivt = 0;
            d.deathstage = 1;
            d.y = d.v * 18 + 18;
            d.notfiring = true;
            d.emocttime = 0;
            d.firepressed = false;
            d.expsn = 0;
            d.rechargetime = 0;
            d.emn = 0;
            d.msc = 1;

            let spr = FIRSTDIGGER + (dig - self.curplayer) as usize;
            self.sys.sprites.movedrawspr(spr, d.x, d.y);
        }
        self.digvisible = true;
        self.bonusvisible = false;
        self.bonusmode = false;
    }

    pub fn dodigger(&mut self) {
        self.sys.newframe();
        if self.gauntlet {
            self.drawlives();
            if self.cgtime < self.sys.clock.ftime as i64 {
                self.timeout = true;
            }
            self.cgtime -= self.sys.clock.ftime as i64;
        }
        for n in self.curplayer..self.diggers + self.curplayer {
            let un = n as usize;
            if self.digdat[un].expsn != 0 {
                self.drawexplosion(un);
            } else {
                self.updatefire(un);
            }
            if self.digvisible {
                if self.digdat[un].alive {
                    if self.digdat[un].bagtime != 0 {
                        let (mdir, x, y) = {
                            let d = &self.digdat[un];
                            (d.mdir, d.x, d.y)
                        };
                        let f = self.digdat[un].notfiring && self.digdat[un].rechargetime == 0;
                        self.drawdig(un, mdir, x, y, f);
                        self.incpenalty();
                        self.digdat[un].bagtime -= 1;
                    } else {
                        self.updatedigger(un);
                    }
                } else {
                    self.diggerdie(un);
                }
            }
            if self.digdat[un].emocttime > 0 {
                self.digdat[un].emocttime -= 1;
            }
        }
        if self.bonusmode && self.isalive() {
            if self.bonustimeleft != 0 {
                self.bonustimeleft -= 1;
                if self.startbonustimeleft != 0 || self.bonustimeleft < 20 {
                    self.startbonustimeleft -= 1;
                    if self.bonustimeleft & 1 != 0 {
                        self.sys.screen.set_intensity(0);
                    } else {
                        self.sys.screen.set_intensity(1);
                    }
                    self.sys.sound_bonus();
                    if self.startbonustimeleft == 0 {
                        self.sys.music(0);
                        self.sys.sound_bonus_off();
                        self.sys.screen.set_intensity(1);
                    }
                }
            } else {
                self.endbonusmode();
                self.sys.sound_bonus_off();
                self.sys.music(1);
            }
        }
        if self.bonusmode && !self.isalive() {
            self.endbonusmode();
            self.sys.sound_bonus_off();
            self.sys.music(1);
        }
    }

    fn drawdig(&mut self, n: usize, d: i32, x: i32, y: i32, f: bool) {
        let rel = n - self.curplayer as usize;
        self.drawdigger(rel, d, x, y, f);
        if self.digdat[n].invin {
            self.digdat[n].ivt -= 1;
            if self.digdat[n].ivt == 0 {
                self.digdat[n].invin = false;
            } else if self.digdat[n].ivt % 10 < 5 {
                self.sys.sprites.erasespr(FIRSTDIGGER + rel);
            }
        }
    }

    pub fn drawdigger(&mut self, n: usize, t: i32, x: i32, y: i32, f: bool) {
        let nn = if n == 0 { 0 } else { 31 };
        self.digspr[n] += self.digspd[n];
        if self.digspr[n] == 2 || self.digspr[n] == 0 {
            self.digspd[n] = -self.digspd[n];
        }
        if self.digspr[n] > 2 {
            self.digspr[n] = 2;
        }
        if self.digspr[n] < 0 {
            self.digspr[n] = 0;
        }
        if (0..=6).contains(&t) && t & 1 == 0 {
            let ch = (t + if f { 0 } else { 1 }) * 3 + self.digspr[n] + 1 + nn;
            let idx = FIRSTDIGGER + n;
            self.sys.sprites.initspr(idx, ch, 4, 15, 0, 0);
            self.sys.sprites.drawspr(idx, x, y);
            return;
        }
        if (10..=15).contains(&t) {
            let ch = 40 + nn - t;
            let idx = FIRSTDIGGER + n;
            self.sys.sprites.initspr(idx, ch, 4, 15, 0, 0);
            self.sys.sprites.drawspr(idx, x, y);
            return;
        }
        self.sys.sprites.first = [-1; TYPES];
    }

    pub fn updatefire(&mut self, n: usize) {
        if self.digdat[n].notfiring {
            if self.digdat[n].rechargetime != 0 {
                self.digdat[n].rechargetime -= 1;
            } else if self.getfirepflag(n as i32 - self.curplayer) {
                if self.digdat[n].alive {
                    self.digdat[n].rechargetime = self.levof10() * 3 + 60;
                    self.digdat[n].notfiring = false;
                    match self.digdat[n].dir {
                        DIR_RIGHT => {
                            self.digdat[n].fx = self.digdat[n].x + 8;
                            self.digdat[n].fy = self.digdat[n].y + 4;
                        }
                        DIR_UP => {
                            self.digdat[n].fx = self.digdat[n].x + 4;
                            self.digdat[n].fy = self.digdat[n].y;
                        }
                        DIR_LEFT => {
                            self.digdat[n].fx = self.digdat[n].x;
                            self.digdat[n].fy = self.digdat[n].y + 4;
                        }
                        DIR_DOWN => {
                            self.digdat[n].fx = self.digdat[n].x + 4;
                            self.digdat[n].fy = self.digdat[n].y + 8;
                        }
                        _ => {}
                    }
                    self.digdat[n].fdir = self.digdat[n].dir;
                    let spr = FIRSTFIREBALL + (n - self.curplayer as usize);
                    let (fx, fy) = (self.digdat[n].fx, self.digdat[n].fy);
                    self.sys.sprites.movedrawspr(spr, fx, fy);
                    self.sys.sound_fire(n);
                }
            }
        } else {
            let fdir = self.digdat[n].fdir;
            let fx = self.digdat[n].fx;
            let fy = self.digdat[n].fy;
            let pix = match fdir {
                DIR_RIGHT => {
                    self.digdat[n].fx += 8;
                    let fx = self.digdat[n].fx;
                    self.sys.screen.getpix(fx, fy + 4) | self.sys.screen.getpix(fx + 4, fy + 4)
                }
                DIR_UP => {
                    self.digdat[n].fy -= 7;
                    let fy = self.digdat[n].fy;
                    let mut pix = 0;
                    for i in 0..7 {
                        pix |= self.sys.screen.getpix(fx + 4, fy + i);
                    }
                    pix & 0xc0
                }
                DIR_LEFT => {
                    self.digdat[n].fx -= 8;
                    let fx = self.digdat[n].fx;
                    self.sys.screen.getpix(fx, fy + 4) | self.sys.screen.getpix(fx + 4, fy + 4)
                }
                DIR_DOWN => {
                    self.digdat[n].fy += 7;
                    let fy = self.digdat[n].fy;
                    let mut pix = 0;
                    for i in 0..7 {
                        pix |= self.sys.screen.getpix(fx, fy + i);
                    }
                    pix & 0x03
                }
                _ => 0,
            };
            let rel = n - self.curplayer as usize;
            let (fx, fy) = (self.digdat[n].fx, self.digdat[n].fy);
            self.drawfire(rel, fx, fy, 0);
            let clfirst = self.clfirst();
            let clcoll = self.clcoll();
            self.incpenalty();

            let mut i = clfirst[2];
            while i != -1 {
                self.killmon(i - FIRSTMONSTER as i32);
                self.scorekill(n);
                self.digdat[n].expsn = 1;
                i = clcoll[i as usize];
            }
            let mut i = clfirst[4];
            while i != -1 {
                let d = (i - FIRSTDIGGER as i32 + self.curplayer) as usize;
                if d != n && !self.digdat[d].invin && self.digdat[d].alive {
                    self.killdigger(d, 3, 0);
                    self.digdat[n].expsn = 1;
                }
                i = clcoll[i as usize];
            }
            let clflag = clfirst.iter().any(|&x| x != -1);
            if clfirst[0] != -1 || clfirst[1] != -1 || clfirst[3] != -1 {
                self.digdat[n].expsn = 1;
                let mut i = clfirst[3];
                while i != -1 {
                    let f = (i - FIRSTFIREBALL as i32 + self.curplayer) as usize;
                    if self.digdat[f].expsn == 0 {
                        self.digdat[f].expsn = 1;
                    }
                    i = clcoll[i as usize];
                }
            }
            match fdir {
                DIR_RIGHT => {
                    if self.digdat[n].fx > 296 {
                        self.digdat[n].expsn = 1;
                    } else if pix != 0 && !clflag {
                        self.digdat[n].expsn = 1;
                        self.digdat[n].fx -= 8;
                        let (fx, fy) = (self.digdat[n].fx, self.digdat[n].fy);
                        self.drawfire(rel, fx, fy, 0);
                    }
                }
                DIR_UP => {
                    if self.digdat[n].fy < 15 {
                        self.digdat[n].expsn = 1;
                    } else if pix != 0 && !clflag {
                        self.digdat[n].expsn = 1;
                        self.digdat[n].fy += 7;
                        let (fx, fy) = (self.digdat[n].fx, self.digdat[n].fy);
                        self.drawfire(rel, fx, fy, 0);
                    }
                }
                DIR_LEFT => {
                    if self.digdat[n].fx < 16 {
                        self.digdat[n].expsn = 1;
                    } else if pix != 0 && !clflag {
                        self.digdat[n].expsn = 1;
                        self.digdat[n].fx += 8;
                        let (fx, fy) = (self.digdat[n].fx, self.digdat[n].fy);
                        self.drawfire(rel, fx, fy, 0);
                    }
                }
                DIR_DOWN => {
                    if self.digdat[n].fy > 183 {
                        self.digdat[n].expsn = 1;
                    } else if pix != 0 && !clflag {
                        self.digdat[n].expsn = 1;
                        self.digdat[n].fy -= 7;
                        let (fx, fy) = (self.digdat[n].fx, self.digdat[n].fy);
                        self.drawfire(rel, fx, fy, 0);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn drawfire(&mut self, n: usize, x: i32, y: i32, t: i32) {
        let nn = if n == 0 { 0 } else { 32 };
        let ch = if t == 0 {
            self.firespr[n] += 1;
            if self.firespr[n] > 2 {
                self.firespr[n] = 0;
            }
            82 + self.firespr[n] + nn
        } else {
            84 + t + nn
        };
        let idx = FIRSTFIREBALL + n;
        self.sys.sprites.initspr(idx, ch, 2, 8, 0, 0);
        self.sys.sprites.drawspr(idx, x, y);
    }

    pub fn drawexplosion(&mut self, n: usize) {
        match self.digdat[n].expsn {
            1 => {
                self.sys.sound_explode(n);
                self.drawfire(
                    n - self.curplayer as usize,
                    self.digdat[n].fx,
                    self.digdat[n].fy,
                    1,
                );
                self.incpenalty();
                self.digdat[n].expsn += 1;
            }
            2 | 3 => {
                self.drawfire(
                    n - self.curplayer as usize,
                    self.digdat[n].fx,
                    self.digdat[n].fy,
                    self.digdat[n].expsn,
                );
                self.incpenalty();
                self.digdat[n].expsn += 1;
            }
            _ => {
                self.killfire(n);
                self.digdat[n].expsn = 0;
            }
        }
    }

    pub fn killfire(&mut self, n: usize) {
        if !self.digdat[n].notfiring {
            self.digdat[n].notfiring = true;
            let spr = FIRSTFIREBALL + (n - self.curplayer as usize);
            self.sys.sprites.erasespr(spr);
            self.sys.sound_fire_off(n);
        }
    }

    pub fn erasediggers(&mut self) {
        for i in 0..self.diggers {
            self.sys.sprites.erasespr(FIRSTDIGGER + i as usize);
        }
        self.digvisible = false;
    }

    pub fn updatedigger(&mut self, n: usize) {
        let rel = n - self.curplayer as usize;
        self.sys.readdir(rel);
        let dir = self.sys.getdir(rel);
        let ddir = if dir == DIR_RIGHT || dir == DIR_UP || dir == DIR_LEFT || dir == DIR_DOWN {
            dir
        } else {
            DIR_NONE
        };
        if self.digdat[n].rx == 0 && (ddir == DIR_UP || ddir == DIR_DOWN) {
            self.digdat[n].dir = ddir;
            self.digdat[n].mdir = ddir;
        }
        if self.digdat[n].ry == 0 && (ddir == DIR_RIGHT || ddir == DIR_LEFT) {
            self.digdat[n].dir = ddir;
            self.digdat[n].mdir = ddir;
        }
        if dir == DIR_NONE {
            self.digdat[n].mdir = DIR_NONE;
        } else {
            self.digdat[n].mdir = self.digdat[n].dir;
        }
        if (self.digdat[n].x == 292 && self.digdat[n].mdir == DIR_RIGHT)
            || (self.digdat[n].x == 12 && self.digdat[n].mdir == DIR_LEFT)
            || (self.digdat[n].y == 180 && self.digdat[n].mdir == DIR_DOWN)
            || (self.digdat[n].y == 18 && self.digdat[n].mdir == DIR_UP)
        {
            self.digdat[n].mdir = DIR_NONE;
        }
        let diggerox = self.digdat[n].x;
        let diggeroy = self.digdat[n].y;
        if self.digdat[n].mdir != DIR_NONE {
            self.eatfield(diggerox, diggeroy, self.digdat[n].mdir);
        }
        match self.digdat[n].mdir {
            DIR_RIGHT => {
                let (x, y) = (self.digdat[n].x, self.digdat[n].y);
                self.sys.screen.draw_right_blob(x, y);
                self.digdat[n].x += 4;
            }
            DIR_UP => {
                let (x, y) = (self.digdat[n].x, self.digdat[n].y);
                self.sys.screen.draw_top_blob(x, y);
                self.digdat[n].y -= 3;
            }
            DIR_LEFT => {
                let (x, y) = (self.digdat[n].x, self.digdat[n].y);
                self.sys.screen.draw_left_blob(x, y);
                self.digdat[n].x -= 4;
            }
            DIR_DOWN => {
                let (x, y) = (self.digdat[n].x, self.digdat[n].y);
                self.sys.screen.draw_bottom_blob(x, y);
                self.digdat[n].y += 3;
            }
            _ => {}
        }
        let (hx, hy, rx, ry) = (
            (self.digdat[n].x - 12) / 20,
            (self.digdat[n].y - 18) / 18,
            (self.digdat[n].x - 12) % 20,
            (self.digdat[n].y - 18) % 18,
        );
        let mdir = self.digdat[n].mdir;
        if self.hitemerald(hx, hy, rx, ry, mdir) {
            if self.digdat[n].emocttime == 0 {
                self.digdat[n].emn = 0;
            }
            self.scoreemerald(n);
            self.sys.sound_em();
            self.sys.sound_emerald(self.digdat[n].emn as usize);
            self.digdat[n].emn += 1;
            if self.digdat[n].emn == 8 {
                self.digdat[n].emn = 0;
                self.scoreoctave(n);
            }
            self.digdat[n].emocttime = 9;
        }
        let (x, y, dir2) = (self.digdat[n].x, self.digdat[n].y, self.digdat[n].dir);
        let f = self.digdat[n].notfiring && self.digdat[n].rechargetime == 0;
        self.drawdig(n, dir2, x, y, f);

        let clfirst = self.clfirst();
        let clcoll = self.clcoll();
        self.incpenalty();

        let mut bagf = false;
        let mut i = clfirst[1];
        while i != -1 {
            if self.bagexist(i - FIRSTBAG as i32) {
                bagf = true;
                break;
            }
            i = clcoll[i as usize];
        }

        let mut push = true;
        if bagf {
            let mdir = self.digdat[n].mdir;
            if mdir == DIR_RIGHT || mdir == DIR_LEFT {
                push = self.pushbags(mdir, &clfirst, &clcoll);
                self.digdat[n].bagtime += 1;
            } else if !self.pushudbags(&clfirst, &clcoll) {
                push = false;
            }
            if !push {
                self.digdat[n].x = diggerox;
                self.digdat[n].y = diggeroy;
                let (x, y, d) = (self.digdat[n].x, self.digdat[n].y, self.digdat[n].mdir);
                let f = self.digdat[n].notfiring && self.digdat[n].rechargetime == 0;
                self.drawdig(n, d, x, y, f);
                self.incpenalty();
                let mdir = self.digdat[n].mdir;
                self.digdat[n].dir = self.reversedir(mdir);
            }
        }
        if clfirst[2] != -1 && self.bonusmode && self.digdat[n].alive {
            let mut nmon = self.killmonsters(&clfirst, &clcoll);
            while nmon != 0 {
                self.sys.sound_eat_monster();
                self.sceatm(n);
                nmon -= 1;
            }
        }
        if clfirst[0] != -1 {
            self.scorebonus(n);
            self.initbonusmode();
        }
        self.digdat[n].h = (self.digdat[n].x - 12) / 20;
        self.digdat[n].rx = (self.digdat[n].x - 12) % 20;
        self.digdat[n].v = (self.digdat[n].y - 18) / 18;
        self.digdat[n].ry = (self.digdat[n].y - 18) % 18;
    }

    pub(crate) fn sceatm(&mut self, n: usize) {
        let msc = self.digdat[n].msc;
        self.scoreeatm(n, msc);
        self.digdat[n].msc <<= 1;
    }

    pub fn diggerdie(&mut self, n: usize) {
        let rel = n - self.curplayer as usize;
        match self.digdat[n].deathstage {
            1 => {
                let bag = self.digdat[n].deathbag;
                let by = self.bagy(bag);
                if by + 6 > self.digdat[n].y {
                    self.digdat[n].y = by + 6;
                }
                let (x, y) = (self.digdat[n].x, self.digdat[n].y);
                self.drawdigger(rel, 15, x, y, false);
                self.incpenalty();
                if self.getbagdir(self.digdat[n].deathbag) == -1 {
                    self.sys.sound_digger_die();
                    self.digdat[n].deathtime = 5;
                    self.digdat[n].deathstage = 2;
                    self.digdat[n].deathani = 0;
                    self.digdat[n].y -= 6;
                }
            }
            2 => {
                if self.digdat[n].deathtime != 0 {
                    self.digdat[n].deathtime -= 1;
                    return;
                }
                if self.digdat[n].deathani == 0 {
                    self.sys.music(2);
                }
                let (x, y, ani) = (self.digdat[n].x, self.digdat[n].y, self.digdat[n].deathani);
                self.drawdigger(rel, 14 - ani, x, y, false);
                let clfirst = self.clfirst();
                let clcoll = self.clcoll();
                self.incpenalty();
                if self.digdat[n].deathani == 0 && clfirst[2] != -1 {
                    self.killmonsters(&clfirst, &clcoll);
                }
                if self.digdat[n].deathani < 4 {
                    self.digdat[n].deathani += 1;
                    self.digdat[n].deathtime = 2;
                } else {
                    self.digdat[n].deathstage = 4;
                    self.digdat[n].deathtime = if self.sys.music_enabled || self.diggers > 1 {
                        60
                    } else {
                        10
                    };
                }
            }
            3 => {
                self.digdat[n].deathstage = 5;
                self.digdat[n].deathani = 0;
                self.digdat[n].deathtime = 0;
            }
            5 => {
                if (0..=6).contains(&self.digdat[n].deathani) {
                    let y = self.digdat[n].y - DEATHARC[self.digdat[n].deathani as usize];
                    let (x, ani) = (self.digdat[n].x, self.digdat[n].deathani);
                    self.drawdigger(rel, 15, x, y, false);
                    if ani == 6 && !self.isalive() {
                        self.sys.music_off();
                    }
                    self.incpenalty();
                    self.digdat[n].deathani += 1;
                    if self.digdat[n].deathani == 1 {
                        self.sys.sound_digger_die();
                    }
                    if self.digdat[n].deathani == 7 {
                        self.digdat[n].deathtime = 5;
                        self.digdat[n].deathani = 0;
                        self.digdat[n].deathstage = 2;
                    }
                }
            }
            4 => {
                if self.digdat[n].deathtime != 0 {
                    self.digdat[n].deathtime -= 1;
                } else {
                    self.digdat[n].dead = true;
                    let mut alldead = true;
                    for i in 0..self.diggers {
                        if !self.digdat[i as usize].dead {
                            alldead = false;
                            break;
                        }
                    }
                    if alldead {
                        self.alldead = true;
                    } else if self.isalive() && self.digdat[n].lives > 0 {
                        if !self.gauntlet {
                            self.digdat[n].lives -= 1;
                        }
                        self.drawlives();
                        if self.digdat[n].lives > 0 {
                            let d = &mut self.digdat[n];
                            d.v = 9;
                            d.mdir = 4;
                            d.h = if self.diggers == 1 {
                                7
                            } else {
                                8 - n as i32 * 2
                            };
                            d.x = d.h * 20 + 12;
                            d.dir = if n == 0 { DIR_RIGHT } else { DIR_LEFT };
                            d.rx = 0;
                            d.ry = 0;
                            d.bagtime = 0;
                            d.alive = true;
                            d.dead = false;
                            d.invin = true;
                            d.ivt = 50;
                            d.deathstage = 1;
                            d.y = d.v * 18 + 18;
                            d.notfiring = true;
                            d.emocttime = 0;
                            d.firepressed = false;
                            d.expsn = 0;
                            d.rechargetime = 0;
                            d.emn = 0;
                            d.msc = 1;
                            self.sys.sprites.erasespr(FIRSTDIGGER + rel);
                            let (x, y) = (d.x, d.y);
                            self.sys.sprites.movedrawspr(FIRSTDIGGER + rel, x, y);
                        }
                        self.sys.clearfire(rel);
                        if self.bonusmode {
                            self.sys.music(0);
                        } else {
                            self.sys.music(1);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn createbonus(&mut self) {
        self.bonusvisible = true;
        self.drawbonus(292, 18);
    }

    pub fn initbonusmode(&mut self) {
        self.bonusmode = true;
        self.erasebonus();
        self.sys.screen.set_intensity(1);
        self.bonustimeleft = 250 - self.levof10() * 20;
        self.startbonustimeleft = 20;
        for i in 0..self.diggers as usize {
            self.digdat[i].msc = 1;
        }
    }

    pub fn endbonusmode(&mut self) {
        self.bonusmode = false;
        self.sys.screen.set_intensity(0);
    }

    pub fn erasebonus(&mut self) {
        if self.bonusvisible {
            self.bonusvisible = false;
            self.sys.sprites.erasespr(FIRSTBONUS);
        }
        self.sys.screen.set_intensity(0);
    }

    pub fn reversedir(&self, dir: i32) -> i32 {
        match dir {
            DIR_RIGHT => DIR_LEFT,
            DIR_LEFT => DIR_RIGHT,
            DIR_UP => DIR_DOWN,
            DIR_DOWN => DIR_UP,
            _ => dir,
        }
    }

    pub fn checkdiggerunderbag(&self, h: i32, v: i32) -> bool {
        for n in self.curplayer..self.diggers + self.curplayer {
            let d = &self.digdat[n as usize];
            if d.alive
                && (d.mdir == DIR_UP || d.mdir == DIR_DOWN)
                && (d.x - 12) / 20 == h
                && (((d.y - 18) / 18 == v) || ((d.y - 18) / 18 + 1 == v))
            {
                return true;
            }
        }
        false
    }

    pub fn killdigger(&mut self, n: usize, stage: i32, bag: i32) {
        if self.digdat[n].invin {
            return;
        }
        if self.digdat[n].deathstage < 2 || self.digdat[n].deathstage > 4 {
            self.digdat[n].alive = false;
            self.digdat[n].deathstage = stage;
            self.digdat[n].deathbag = bag;
        }
    }

    pub fn eatfield(&mut self, x: i32, y: i32, dir: i32) {
        let mut h = (x - 12) / 20;
        let xr = ((x - 12) % 20) / 4;
        let mut v = (y - 18) / 18;
        let mut yr = ((y - 18) % 18) / 3;
        self.incpenalty();
        match dir {
            DIR_RIGHT => {
                h += 1;
                let idx = (v * 15 + h) as usize;
                if idx < MSIZE {
                    self.field[idx] &= BITMASKS[xr as usize];
                    if self.field[idx] & 0x1f == 0 {
                        self.field[idx] &= 0xdfff;
                    }
                }
            }
            DIR_UP => {
                yr -= 1;
                if yr < 0 {
                    yr += 6;
                    v -= 1;
                }
                let idx = (v * 15 + h) as usize;
                if idx < MSIZE {
                    self.field[idx] &= BITMASKS[(6 + yr) as usize];
                    if self.field[idx] & 0xfc0 == 0 {
                        self.field[idx] &= 0xdfff;
                    }
                }
            }
            DIR_LEFT => {
                let mut xr2 = xr - 1;
                if xr2 < 0 {
                    xr2 += 5;
                    h -= 1;
                }
                let idx = (v * 15 + h) as usize;
                if idx < MSIZE {
                    self.field[idx] &= BITMASKS[xr2 as usize];
                    if self.field[idx] & 0x1f == 0 {
                        self.field[idx] &= 0xdfff;
                    }
                }
            }
            DIR_DOWN => {
                v += 1;
                let idx = (v * 15 + h) as usize;
                if idx < MSIZE {
                    self.field[idx] &= BITMASKS[(6 + yr) as usize];
                    if self.field[idx] & 0xfc0 == 0 {
                        self.field[idx] &= 0xdfff;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn getfirepflag(&self, n: i32) -> bool {
        if n == 0 {
            self.sys.getfire(0)
        } else {
            self.sys.getfire(1)
        }
    }

    pub fn diggerx(&self, n: usize) -> i32 {
        self.digdat[n].x
    }
    pub fn diggery(&self, n: usize) -> i32 {
        self.digdat[n].y
    }
    pub fn digalive(&self, n: i32) -> bool {
        if !(0..2).contains(&n) {
            return false;
        }
        self.digdat[n as usize].alive
    }
    pub fn digresettime(&mut self, n: usize) {
        self.digdat[n].bagtime = 0;
    }
    pub fn isalive(&self) -> bool {
        for i in self.curplayer..self.diggers + self.curplayer {
            if self.digdat[i as usize].alive {
                return true;
            }
        }
        false
    }
    pub fn getlives(&self, pl: i32) -> i32 {
        self.digdat[pl as usize].lives
    }
    pub fn addlife(&mut self, pl: i32) {
        self.digdat[pl as usize].lives += 1;
        self.sys.sound_one_up();
    }
    pub fn initlives(&mut self) {
        for i in 0..self.diggers + self.nplayers - 1 {
            self.digdat[i as usize].lives = START_LIVES;
        }
    }
    pub fn declife(&mut self, pl: i32) {
        if !self.gauntlet {
            self.digdat[pl as usize].lives -= 1;
        }
    }
}
