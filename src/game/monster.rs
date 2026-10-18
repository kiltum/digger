//! Port of `monster.c`: Nobbin/Hobbin AI, monster creation and deaths.

use crate::constants::*;
use crate::game::Game;
use crate::sprites::*;

impl Game {
    pub fn initmonsters(&mut self) {
        for m in self.mondat.iter_mut() {
            m.flag = false;
        }
        self.nextmonster = 0;
        self.mongaptime = 45 - (self.levof10() << 1);
        self.totalmonsters = self.levof10() + 5;
        self.maxmononscr = match self.levof10() {
            1 => 3,
            2..=7 => 4,
            _ => 5,
        };
        self.nextmontime = 10;
        self.unbonusflag = true;
    }

    pub fn erasemonsters(&mut self) {
        for i in 0..MONSTERS {
            if self.mondat[i].flag {
                self.sys.sprites.erasespr(i + FIRSTMONSTER);
            }
        }
    }

    pub fn domonsters(&mut self) {
        if self.nextmontime > 0 {
            self.nextmontime -= 1;
        } else {
            if self.nextmonster < self.totalmonsters
                && self.nmononscr() < self.maxmononscr
                && self.isalive()
                && !self.bonusmode
            {
                self.createmonster();
            }
            if self.unbonusflag && self.nextmonster == self.totalmonsters && self.nextmontime == 0 {
                if self.isalive() {
                    self.unbonusflag = false;
                    self.createbonus();
                }
            }
        }
        for i in 0..MONSTERS {
            if !self.mondat[i].flag {
                continue;
            }
            if self.mondat[i].hnt > 10 - self.levof10() {
                if self.mondat[i].nob {
                    self.mondat[i].nob = false;
                    self.mondat[i].hnt = 0;
                }
            }
            if self.mondat[i].alive {
                if self.mondat[i].t == 0 {
                    self.monai(i);
                    let r = self.randno(15 - self.levof10());
                    if r == 0 && self.mondat[i].nob && self.mondat[i].alive {
                        self.monai(i);
                    }
                } else {
                    self.mondat[i].t -= 1;
                }
            } else {
                self.mondie(i);
            }
        }
    }

    fn createmonster(&mut self) {
        for i in 0..MONSTERS {
            if !self.mondat[i].flag {
                self.mondat[i].flag = true;
                self.mondat[i].alive = true;
                self.mondat[i].t = 0;
                self.mondat[i].nob = true;
                self.mondat[i].hnt = 0;
                self.mondat[i].h = 14;
                self.mondat[i].v = 0;
                self.mondat[i].x = 292;
                self.mondat[i].y = 18;
                self.mondat[i].xr = 0;
                self.mondat[i].yr = 0;
                self.mondat[i].dir = DIR_LEFT;
                self.mondat[i].hdir = DIR_LEFT;
                self.mondat[i].chase = self.chase + self.curplayer;
                self.chase = (self.chase + 1) % self.diggers;
                self.nextmonster += 1;
                self.nextmontime = self.mongaptime;
                self.mondat[i].stime = 5;
                self.sys
                    .sprites
                    .movedrawspr(i + FIRSTMONSTER, self.mondat[i].x, self.mondat[i].y);
                break;
            }
        }
    }

    pub fn mongold(&mut self) {
        self.mongotgold = true;
    }

    fn monai(&mut self, mon: usize) {
        let monox = self.mondat[mon].x;
        let monoy = self.mondat[mon].y;
        if self.mondat[mon].xr == 0 && self.mondat[mon].yr == 0 {
            if self.mondat[mon].hnt > 30 + (self.levof10() << 1) {
                if !self.mondat[mon].nob {
                    self.mondat[mon].hnt = 0;
                    self.mondat[mon].nob = true;
                }
            }

            // The original computes `(diggers - 1) - dig` when the chased
            // digger is dead.  With one active digger and a stale chase slot
            // this yields -1 (out-of-bounds C).  Clamp to the active player.
            let mut dig = self.mondat[mon].chase.clamp(0, 1);
            if !self.digalive(dig) {
                dig = (self.diggers - 1) - dig;
                if !(0..2).contains(&dig) {
                    dig = self.curplayer;
                }
            }

            let (mut mdirp1, mut mdirp2, mut mdirp3, mut mdirp4);
            if (self.diggery(dig as usize) - self.mondat[mon].y).abs()
                > (self.diggerx(dig as usize) - self.mondat[mon].x).abs()
            {
                if self.diggery(dig as usize) < self.mondat[mon].y {
                    mdirp1 = DIR_UP;
                    mdirp4 = DIR_DOWN;
                } else {
                    mdirp1 = DIR_DOWN;
                    mdirp4 = DIR_UP;
                }
                if self.diggerx(dig as usize) < self.mondat[mon].x {
                    mdirp2 = DIR_LEFT;
                    mdirp3 = DIR_RIGHT;
                } else {
                    mdirp2 = DIR_RIGHT;
                    mdirp3 = DIR_LEFT;
                }
            } else {
                if self.diggerx(dig as usize) < self.mondat[mon].x {
                    mdirp1 = DIR_LEFT;
                    mdirp4 = DIR_RIGHT;
                } else {
                    mdirp1 = DIR_RIGHT;
                    mdirp4 = DIR_LEFT;
                }
                if self.diggery(dig as usize) < self.mondat[mon].y {
                    mdirp2 = DIR_UP;
                    mdirp3 = DIR_DOWN;
                } else {
                    mdirp2 = DIR_DOWN;
                    mdirp3 = DIR_UP;
                }
            }

            if self.bonusmode {
                std::mem::swap(&mut mdirp1, &mut mdirp4);
                std::mem::swap(&mut mdirp2, &mut mdirp3);
            }

            let rev = self.reversedir(self.mondat[mon].dir);
            if rev == mdirp1 {
                mdirp1 = mdirp2;
                mdirp2 = mdirp3;
                mdirp3 = mdirp4;
                mdirp4 = rev;
            }
            if rev == mdirp2 {
                mdirp2 = mdirp3;
                mdirp3 = mdirp4;
                mdirp4 = rev;
            }
            if rev == mdirp3 {
                mdirp3 = mdirp4;
                mdirp4 = rev;
            }

            if self.randno(self.levof10() + 5) == 1 && self.levof10() < 6 {
                std::mem::swap(&mut mdirp1, &mut mdirp3);
            }

            let mut dir = rev;
            let (h, v) = (self.mondat[mon].h, self.mondat[mon].v);
            if self.fieldclear(mdirp1, h, v) {
                dir = mdirp1;
            } else if self.fieldclear(mdirp2, h, v) {
                dir = mdirp2;
            } else if self.fieldclear(mdirp3, h, v) {
                dir = mdirp3;
            } else if self.fieldclear(mdirp4, h, v) {
                dir = mdirp4;
            }
            if !self.mondat[mon].nob {
                dir = mdirp1;
            }
            if self.mondat[mon].dir != dir {
                self.mondat[mon].t += 1;
            }
            self.mondat[mon].dir = dir;
        }

        if (self.mondat[mon].x == 292 && self.mondat[mon].dir == DIR_RIGHT)
            || (self.mondat[mon].x == 12 && self.mondat[mon].dir == DIR_LEFT)
            || (self.mondat[mon].y == 180 && self.mondat[mon].dir == DIR_DOWN)
            || (self.mondat[mon].y == 18 && self.mondat[mon].dir == DIR_UP)
        {
            self.mondat[mon].dir = DIR_NONE;
        }

        if self.mondat[mon].dir == DIR_LEFT || self.mondat[mon].dir == DIR_RIGHT {
            self.mondat[mon].hdir = self.mondat[mon].dir;
        }

        if !self.mondat[mon].nob {
            let (x, y, d) = (self.mondat[mon].x, self.mondat[mon].y, self.mondat[mon].dir);
            self.eatfield(x, y, d);
        }

        match self.mondat[mon].dir {
            DIR_RIGHT => {
                if !self.mondat[mon].nob {
                    let (x, y) = (self.mondat[mon].x, self.mondat[mon].y);
                    self.sys.screen.draw_right_blob(x, y);
                }
                self.mondat[mon].x += 4;
            }
            DIR_UP => {
                if !self.mondat[mon].nob {
                    let (x, y) = (self.mondat[mon].x, self.mondat[mon].y);
                    self.sys.screen.draw_top_blob(x, y);
                }
                self.mondat[mon].y -= 3;
            }
            DIR_LEFT => {
                if !self.mondat[mon].nob {
                    let (x, y) = (self.mondat[mon].x, self.mondat[mon].y);
                    self.sys.screen.draw_left_blob(x, y);
                }
                self.mondat[mon].x -= 4;
            }
            DIR_DOWN => {
                if !self.mondat[mon].nob {
                    let (x, y) = (self.mondat[mon].x, self.mondat[mon].y);
                    self.sys.screen.draw_bottom_blob(x, y);
                }
                self.mondat[mon].y += 3;
            }
            _ => {}
        }

        if !self.mondat[mon].nob {
            let (hx, hy, rx, ry, d) = (
                (self.mondat[mon].x - 12) / 20,
                (self.mondat[mon].y - 18) / 18,
                (self.mondat[mon].x - 12) % 20,
                (self.mondat[mon].y - 18) % 18,
                self.mondat[mon].dir,
            );
            self.hitemerald(hx, hy, rx, ry, d);
        }

        if !self.isalive() {
            self.mondat[mon].x = monox;
            self.mondat[mon].y = monoy;
        }
        if self.mondat[mon].stime != 0 {
            self.mondat[mon].stime -= 1;
            self.mondat[mon].x = monox;
            self.mondat[mon].y = monoy;
        }
        if !self.mondat[mon].nob && self.mondat[mon].hnt < 100 {
            self.mondat[mon].hnt += 1;
        }

        let (nob, hdir, x, y) = (
            self.mondat[mon].nob,
            self.mondat[mon].hdir,
            self.mondat[mon].x,
            self.mondat[mon].y,
        );
        self.drawmon(mon, nob, hdir, x, y);
        let clfirst = self.clfirst();
        let clcoll = self.clcoll();
        self.incpenalty();

        if clfirst[2] != -1 {
            self.mondat[mon].t += 1;
            let mut i = clfirst[2];
            loop {
                let m = (i - FIRSTMONSTER as i32) as usize;
                if self.mondat[mon].dir == self.mondat[m].dir
                    && self.mondat[m].stime == 0
                    && self.mondat[mon].stime == 0
                {
                    self.mondat[m].dir = self.reversedir(self.mondat[m].dir);
                }
                self.incpenalty();
                i = clcoll[i as usize];
                if i == -1 {
                    break;
                }
            }
        }

        let mut i = clfirst[1];
        let mut bagf = false;
        while i != -1 {
            if self.bagexist(i - FIRSTBAG as i32) {
                bagf = true;
                break;
            }
            i = clcoll[i as usize];
        }

        let mut push = true;
        if bagf {
            self.mondat[mon].t += 1;
            self.mongotgold = false;
            let dir = self.mondat[mon].dir;
            if dir == DIR_RIGHT || dir == DIR_LEFT {
                push = self.pushbags(dir, &clfirst, &clcoll);
                self.mondat[mon].t += 1;
            } else if !self.pushudbags(&clfirst, &clcoll) {
                push = false;
            }
            if self.mongotgold {
                self.mondat[mon].t = 0;
            }
            if !self.mondat[mon].nob && self.mondat[mon].hnt > 1 {
                self.removebags(&clfirst, &clcoll);
            }
        }

        if self.mondat[mon].nob && clfirst[2] != -1 && self.isalive() {
            self.mondat[mon].hnt += 1;
        }

        if !push {
            self.mondat[mon].x = monox;
            self.mondat[mon].y = monoy;
            let (nob, hdir, x, y) = (
                self.mondat[mon].nob,
                self.mondat[mon].hdir,
                self.mondat[mon].x,
                self.mondat[mon].y,
            );
            self.drawmon(mon, nob, hdir, x, y);
            self.incpenalty();
            if self.mondat[mon].nob {
                self.mondat[mon].hnt += 1;
            }
            let d = self.mondat[mon].dir;
            if (d == DIR_UP || d == DIR_DOWN) && self.mondat[mon].nob {
                self.mondat[mon].dir = self.reversedir(d);
            }
        }

        if clfirst[4] != -1 && self.isalive() {
            if self.bonusmode {
                self.killmon(mon as i32);
                let mut i = clfirst[4];
                while i != -1 {
                    let d = (i - FIRSTDIGGER as i32 + self.curplayer) as usize;
                    if self.digalive(d as i32) {
                        self.sceatm(d);
                    }
                    i = clcoll[i as usize];
                }
                self.sys.sound_eat_monster();
            } else {
                let mut i = clfirst[4];
                while i != -1 {
                    let d = (i - FIRSTDIGGER as i32 + self.curplayer) as usize;
                    if self.digalive(d as i32) {
                        self.killdigger(d, 3, 0);
                    }
                    i = clcoll[i as usize];
                }
            }
        }

        self.mondat[mon].h = (self.mondat[mon].x - 12) / 20;
        self.mondat[mon].v = (self.mondat[mon].y - 18) / 18;
        self.mondat[mon].xr = (self.mondat[mon].x - 12) % 20;
        self.mondat[mon].yr = (self.mondat[mon].y - 18) % 18;
    }

    fn mondie(&mut self, mon: usize) {
        match self.mondat[mon].death {
            1 => {
                let by = self.bagy(self.mondat[mon].bag);
                if by + 6 > self.mondat[mon].y {
                    self.mondat[mon].y = by;
                }
                let (nob, hdir, x, y) = (
                    self.mondat[mon].nob,
                    self.mondat[mon].hdir,
                    self.mondat[mon].x,
                    self.mondat[mon].y,
                );
                self.drawmondie(mon, nob, hdir, x, y);
                self.incpenalty();
                if self.getbagdir(self.mondat[mon].bag) == -1 {
                    self.mondat[mon].dtime = 1;
                    self.mondat[mon].death = 4;
                }
            }
            4 => {
                if self.mondat[mon].dtime != 0 {
                    self.mondat[mon].dtime -= 1;
                } else {
                    self.killmon(mon as i32);
                    if self.diggers == 2 {
                        self.scorekill2();
                    } else {
                        self.scorekill(self.curplayer as usize);
                    }
                }
            }
            _ => {}
        }
    }

    fn fieldclear(&self, dir: i32, x: i32, y: i32) -> bool {
        match dir {
            DIR_RIGHT => {
                if x < 14
                    && (self.getfield(x + 1, y) & 0x2000) == 0
                    && ((self.getfield(x + 1, y) & 1) == 0 || (self.getfield(x, y) & 0x10) == 0)
                {
                    return true;
                }
            }
            DIR_UP => {
                if y > 0
                    && (self.getfield(x, y - 1) & 0x2000) == 0
                    && ((self.getfield(x, y - 1) & 0x800) == 0 || (self.getfield(x, y) & 0x40) == 0)
                {
                    return true;
                }
            }
            DIR_LEFT => {
                if x > 0
                    && (self.getfield(x - 1, y) & 0x2000) == 0
                    && ((self.getfield(x - 1, y) & 0x10) == 0 || (self.getfield(x, y) & 1) == 0)
                {
                    return true;
                }
            }
            DIR_DOWN => {
                if y < 9
                    && (self.getfield(x, y + 1) & 0x2000) == 0
                    && ((self.getfield(x, y + 1) & 0x40) == 0 || (self.getfield(x, y) & 0x800) == 0)
                {
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    pub fn checkmonscared(&mut self, h: i32) {
        for m in 0..MONSTERS {
            if h == self.mondat[m].h && self.mondat[m].dir == DIR_UP {
                self.mondat[m].dir = DIR_DOWN;
            }
        }
    }

    pub fn killmon(&mut self, mon: i32) {
        let m = mon as usize;
        if self.mondat[m].flag {
            self.mondat[m].flag = false;
            self.mondat[m].alive = false;
            self.sys.sprites.erasespr(m + FIRSTMONSTER);
            if self.bonusmode {
                self.totalmonsters += 1;
            }
        }
    }

    pub fn squashmonsters(&mut self, bag: i32, clfirst: &[i32; TYPES], clcoll: &[i32; SPRITES]) {
        let mut next = clfirst[2];
        while next != -1 {
            let m = (next - FIRSTMONSTER as i32) as usize;
            if self.mondat[m].y >= self.bagy(bag) {
                self.squashmonster(m, 1, bag);
            }
            next = clcoll[next as usize];
        }
    }

    pub fn killmonsters(&mut self, clfirst: &[i32; TYPES], clcoll: &[i32; SPRITES]) -> i32 {
        let mut next = clfirst[2];
        let mut n = 0;
        while next != -1 {
            let m = next - FIRSTMONSTER as i32;
            self.killmon(m);
            n += 1;
            next = clcoll[next as usize];
        }
        n
    }

    fn squashmonster(&mut self, mon: usize, death: i32, bag: i32) {
        self.mondat[mon].alive = false;
        self.mondat[mon].death = death;
        self.mondat[mon].bag = bag;
    }

    pub fn monleft(&self) -> i32 {
        self.nmononscr() + self.totalmonsters - self.nextmonster
    }

    fn nmononscr(&self) -> i32 {
        self.mondat.iter().filter(|m| m.flag).count() as i32
    }

    pub fn incmont(&mut self, n: i32) {
        let n = n.min(MONSTERS as i32);
        for m in 1..n {
            self.mondat[m as usize].t += 1;
        }
    }

    pub fn drawmon(&mut self, n: usize, nobf: bool, dir: i32, x: i32, y: i32) {
        self.monspr[n] += self.monspd[n];
        if self.monspr[n] == 2 || self.monspr[n] == 0 {
            self.monspd[n] = -self.monspd[n];
        }
        if self.monspr[n] > 2 {
            self.monspr[n] = 2;
        }
        if self.monspr[n] < 0 {
            self.monspr[n] = 0;
        }
        let idx = FIRSTMONSTER + n;
        if nobf {
            self.sys
                .sprites
                .initspr(idx, self.monspr[n] + 69, 4, 15, 0, 0);
        } else {
            match dir {
                DIR_RIGHT => {
                    self.sys
                        .sprites
                        .initspr(idx, self.monspr[n] + 73, 4, 15, 0, 0);
                }
                DIR_LEFT => {
                    self.sys
                        .sprites
                        .initspr(idx, self.monspr[n] + 77, 4, 15, 0, 0);
                }
                _ => {}
            }
        }
        self.sys.sprites.drawspr(idx, x, y);
    }

    fn drawmondie(&mut self, n: usize, nobf: bool, dir: i32, x: i32, y: i32) {
        let idx = FIRSTMONSTER + n;
        if nobf {
            self.sys.sprites.initspr(idx, 72, 4, 15, 0, 0);
        } else {
            match dir {
                DIR_RIGHT => {
                    self.sys.sprites.initspr(idx, 76, 4, 15, 0, 0);
                }
                DIR_LEFT => {
                    self.sys.sprites.initspr(idx, 80, 4, 14, 0, 0);
                }
                _ => {}
            }
        }
        self.sys.sprites.drawspr(idx, x, y);
    }
}
