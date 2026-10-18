//! Game state and the top-level flow ported from `main.c`.

pub mod bags;
pub mod digger;
pub mod monster;
pub mod scores;

use crate::constants::*;
use crate::levels::LEVELS;
use crate::sprites::*;
use crate::sys::Sys;

#[derive(Clone, Copy, Debug)]
pub struct GameDat {
    pub level: i32,
    pub levdone: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Digger {
    pub x: i32,
    pub y: i32,
    pub h: i32,
    pub v: i32,
    pub rx: i32,
    pub ry: i32,
    pub mdir: i32,
    pub dir: i32,
    pub bagtime: i32,
    pub rechargetime: i32,
    pub fx: i32,
    pub fy: i32,
    pub fdir: i32,
    pub expsn: i32,
    pub deathstage: i32,
    pub deathbag: i32,
    pub deathani: i32,
    pub deathtime: i32,
    pub emocttime: i32,
    pub emn: i32,
    pub msc: i32,
    pub lives: i32,
    pub ivt: i32,
    pub notfiring: bool,
    pub alive: bool,
    pub firepressed: bool,
    pub dead: bool,
    pub levdone: bool,
    pub invin: bool,
}

impl Default for Digger {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            h: 0,
            v: 0,
            rx: 0,
            ry: 0,
            mdir: 4,
            dir: 0,
            bagtime: 0,
            rechargetime: 0,
            fx: 0,
            fy: 0,
            fdir: 0,
            expsn: 0,
            deathstage: 1,
            deathbag: 0,
            deathani: 0,
            deathtime: 0,
            emocttime: 0,
            emn: 0,
            msc: 1,
            lives: 0,
            ivt: 0,
            notfiring: true,
            alive: false,
            firepressed: false,
            dead: false,
            levdone: false,
            invin: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Monster {
    pub x: i32,
    pub y: i32,
    pub h: i32,
    pub v: i32,
    pub xr: i32,
    pub yr: i32,
    pub dir: i32,
    pub hdir: i32,
    pub t: i32,
    pub hnt: i32,
    pub death: i32,
    pub bag: i32,
    pub dtime: i32,
    pub stime: i32,
    pub chase: i32,
    pub flag: bool,
    pub nob: bool,
    pub alive: bool,
}

impl Default for Monster {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            h: 0,
            v: 0,
            xr: 0,
            yr: 0,
            dir: DIR_LEFT,
            hdir: DIR_LEFT,
            t: 0,
            hnt: 0,
            death: 0,
            bag: 0,
            dtime: 0,
            stime: 0,
            chase: 0,
            flag: false,
            nob: true,
            alive: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Bag {
    pub x: i32,
    pub y: i32,
    pub h: i32,
    pub v: i32,
    pub xr: i32,
    pub yr: i32,
    pub dir: i32,
    pub wt: i32,
    pub gt: i32,
    pub fallh: i32,
    pub wobbling: bool,
    pub unfallen: bool,
    pub exist: bool,
}

impl Default for Bag {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            h: 0,
            v: 0,
            xr: 0,
            yr: 0,
            dir: DIR_NONE,
            wt: 15,
            gt: 0,
            fallh: 0,
            wobbling: false,
            unfallen: true,
            exist: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ScoreDat {
    pub score: i64,
    pub nextbs: i64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HighScore {
    pub name: [u8; 3],
    pub score: i64,
}

pub struct Game {
    pub sys: Sys,

    pub nplayers: i32,
    pub diggers: i32,
    pub curplayer: i32,
    pub startlev: i32,
    pub gamedat: [GameDat; 2],

    pub field: [u16; MSIZE],
    pub field1: [u16; MSIZE],
    pub field2: [u16; MSIZE],
    pub emfield: [u8; MSIZE],
    pub emmask: u8,

    pub digdat: [Digger; 2],
    pub mondat: [Monster; MONSTERS],
    pub bagdat: [Bag; BAGS],
    pub bagdat1: [Bag; BAGS],
    pub bagdat2: [Bag; BAGS],

    pub nextmonster: i32,
    pub totalmonsters: i32,
    pub maxmononscr: i32,
    pub nextmontime: i32,
    pub mongaptime: i32,
    pub chase: i32,
    pub unbonusflag: bool,
    pub mongotgold: bool,

    pub pushcount: i32,
    pub goldtime: i32,

    pub penalty: i32,
    pub alldead: bool,
    pub levnotdrawn: bool,
    pub started: bool,
    pub timeout: bool,
    pub gauntlet: bool,
    pub unlimlives: bool,

    pub bonusvisible: bool,
    pub bonusmode: bool,
    pub digvisible: bool,
    pub bonustimeleft: i32,
    pub startbonustimeleft: i32,

    pub scdat: [ScoreDat; 2],
    pub highscores: [HighScore; 10],
    pub scoret: i64,

    pub randv: i64,
    /// Deterministic RNG seed for tests (`DIGGER_SEED`).
    pub rng_seed: Option<i64>,
    pub frame: u64,
    pub gtime: i32,
    pub cgtime: i64,

    /// Animation state from `drawing.c`.
    pub monspr: [i32; MONSTERS],
    pub monspd: [i32; MONSTERS],
    pub digspr: [i32; DIGGERS],
    pub digspd: [i32; DIGGERS],
    pub firespr: [i32; FIREBALLS],
}

impl Game {
    pub fn new(sys: Sys) -> Self {
        let highscores = scores::load_highscores();
        let startlev = std::env::var("DIGGER_START_LEVEL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        Self {
            sys,
            nplayers: 1,
            diggers: 1,
            curplayer: 0,
            startlev,
            gamedat: [GameDat {
                level: 1,
                levdone: false,
            }; 2],
            field: [0; MSIZE],
            field1: [0; MSIZE],
            field2: [0; MSIZE],
            emfield: [0; MSIZE],
            emmask: 0,
            digdat: [Digger::default(); DIGGERS],
            mondat: [Monster::default(); MONSTERS],
            bagdat: [Bag::default(); BAGS],
            bagdat1: [Bag::default(); BAGS],
            bagdat2: [Bag::default(); BAGS],
            nextmonster: 0,
            totalmonsters: 0,
            maxmononscr: 0,
            nextmontime: 0,
            mongaptime: 0,
            chase: 0,
            unbonusflag: false,
            mongotgold: false,
            pushcount: 0,
            goldtime: 0,
            penalty: 0,
            alldead: false,
            levnotdrawn: false,
            started: false,
            timeout: false,
            gauntlet: false,
            unlimlives: false,
            bonusvisible: false,
            bonusmode: false,
            digvisible: false,
            bonustimeleft: 0,
            startbonustimeleft: 0,
            scdat: [ScoreDat::default(); 2],
            highscores,
            scoret: 0,
            randv: 0,
            rng_seed: std::env::var("DIGGER_SEED")
                .ok()
                .and_then(|s| s.parse().ok()),
            frame: 0,
            gtime: 0,
            cgtime: 0,
            monspr: [0; MONSTERS],
            monspd: [1; MONSTERS],
            digspr: [0; DIGGERS],
            digspd: [1; DIGGERS],
            firespr: [0; FIREBALLS],
        }
    }

    // ------------------------------------------------------------------
    // Top-level flow (`mainprog` + `game`)
    // ------------------------------------------------------------------

    pub fn mainprog(&mut self) {
        loop {
            self.sys.sound_stop();
            self.sys.screen.clear();
            self.sys.sprites = SpriteSystem::new();
            self.creatembspr();
            self.sys.screen.gtitle();
            self.outtext("D I G G E R", 100, 0, 3);
            self.shownplayers();
            self.showtable();
            self.started = false;
            self.frame = 0;
            self.sys.newframe();

            let autostart = std::env::var_os("DIGGER_AUTOSTART").is_some();
            if !autostart {
                let mut x = 292;
                loop {
                    if self.sys.quit {
                        return;
                    }
                    if self.teststart() {
                        break;
                    }

                    if self.frame == 0 {
                        let mut t = 54;
                        while t < 174 {
                            self.outtext("            ", 164, t, 0);
                            t += 12;
                        }
                    }
                    if self.frame == 50 {
                        self.sys.sprites.movedrawspr(FIRSTMONSTER, 292, 63);
                        x = 292;
                    }
                    if self.frame > 50 && self.frame <= 77 {
                        x -= 4;
                        self.drawmon(0, true, DIR_LEFT, x, 63);
                    }
                    if self.frame > 77 {
                        self.drawmon(0, true, DIR_RIGHT, 184, 63);
                    }
                    if self.frame == 83 {
                        self.outtext("NOBBIN", 216, 64, 2);
                    }
                    if self.frame == 90 {
                        self.sys.sprites.movedrawspr(FIRSTMONSTER + 1, 292, 82);
                        self.drawmon(1, false, DIR_LEFT, 292, 82);
                        x = 292;
                    }
                    if self.frame > 90 && self.frame <= 117 {
                        x -= 4;
                        self.drawmon(1, false, DIR_LEFT, x, 82);
                    }
                    if self.frame > 117 {
                        self.drawmon(1, false, DIR_RIGHT, 184, 82);
                    }
                    if self.frame == 123 {
                        self.outtext("HOBBIN", 216, 83, 2);
                    }
                    if self.frame == 130 {
                        self.sys.sprites.movedrawspr(FIRSTDIGGER, 292, 101);
                        self.drawdigger(0, DIR_LEFT, 292, 101, true);
                        x = 292;
                    }
                    if self.frame > 130 && self.frame <= 157 {
                        x -= 4;
                        self.drawdigger(0, DIR_LEFT, x, 101, true);
                    }
                    if self.frame > 157 {
                        self.drawdigger(0, DIR_RIGHT, 184, 101, true);
                    }
                    if self.frame == 163 {
                        self.outtext("DIGGER", 216, 102, 2);
                    }
                    if self.frame == 178 {
                        self.sys.sprites.movedrawspr(FIRSTBAG, 184, 120);
                        self.drawgold(0, 0, 184, 120);
                    }
                    if self.frame == 183 {
                        self.outtext("GOLD", 216, 121, 2);
                    }
                    if self.frame == 198 {
                        self.sys.screen.draw_emerald(184, 141);
                    }
                    if self.frame == 203 {
                        self.outtext("EMERALD", 216, 140, 2);
                    }
                    if self.frame == 218 {
                        self.drawbonus(184, 158);
                    }
                    if self.frame == 223 {
                        self.outtext("BONUS", 216, 159, 2);
                    }

                    self.sys.newframe();
                    self.frame += 1;
                    if self.frame > 250 {
                        self.frame = 0;
                    }
                }
            }

            if self.sys.quit {
                break;
            }
            self.game();
            if self.sys.quit {
                break;
            }
        }
    }

    /// Headless entry point: skip the title/menu, run the simulation until
    /// `Sys` reaches its frame limit (or the game ends).
    pub fn run_headless(&mut self) {
        self.started = true;
        self.game();
    }

    /// Latest composed 0x00RRGGBB frame.
    pub fn frame(&self) -> &[u32] {
        self.sys.screen.frame()
    }

    /// One-line state summary for headless runs.
    pub fn debug_summary(&self) -> String {
        format!(
            "frame={} level={} curplayer={} score={}/{} lives={}/{} monsters={} emeralds={} bags={}",
            self.sys.clock.frame,
            self.levno(),
            self.curplayer,
            self.scdat[0].score,
            self.scdat[1].score,
            self.getlives(0),
            self.getlives(1),
            self.monleft(),
            self.countem(),
            self.bagdat.iter().filter(|b| b.exist).count()
        )
    }

    /// `teststart()` + the title-level player toggle.
    fn teststart(&mut self) -> bool {
        if self.sys.quit {
            return false;
        }
        let mut start = self.sys.input.confirm;
        let chars = self.sys.input.take_chars();
        for c in chars {
            if c == 'n' || c == 'N' {
                if self.diggers == 2 {
                    self.diggers = 1;
                }
                self.nplayers = 3 - self.nplayers;
                self.shownplayers();
            } else if c == 's' || c == 'S' {
                self.diggers = if self.diggers == 2 { 1 } else { 2 };
                self.nplayers = 1;
                self.shownplayers();
            } else if !c.is_control() {
                start = true;
            }
        }
        if self.sys.input.start_key {
            // The original starts on any key except Esc/N.  Function keys also
            // count; this is intentional.
            start = true;
        }
        start
    }

    fn game(&mut self) {
        self.initlives();
        self.gamedat[0].level = self.startlev;
        if self.nplayers == 2 {
            self.gamedat[1].level = self.startlev;
        }
        self.alldead = false;
        self.sys.screen.clear();
        self.curplayer = 0;
        self.initlevel();
        self.curplayer = 1;
        self.initlevel();
        self.zeroscores();
        self.bonusvisible = true;
        let mut flashplayer = self.nplayers == 2;
        self.curplayer = 0;

        while self.getalllives() != 0 && !self.sys.quit && !self.timeout {
            while !self.alldead && !self.sys.quit && !self.timeout {
                self.initmbspr();
                self.seed_rng();
                if self.levnotdrawn {
                    self.levnotdrawn = false;
                    self.drawscreen();
                    if flashplayer {
                        flashplayer = false;
                        let text = if self.curplayer == 0 {
                            "PLAYER 1"
                        } else {
                            "PLAYER 2"
                        };
                        self.cleartopline();
                        for _ in 0..15 {
                            for c in 1..=3u8 {
                                self.outtext(text, 108, 0, c);
                                self.writecurscore(c);
                                self.sys.newframe();
                                if self.sys.quit {
                                    return;
                                }
                            }
                        }
                        self.drawscores();
                        for i in 0..self.diggers {
                            self.addscore(i as usize, 0);
                        }
                    }
                } else {
                    self.initchars();
                }
                self.outtext("        ", 108, 0, 3);
                self.initscores();
                self.drawlives();
                self.sys.music(1);

                self.sys.flush_input_edges();
                for i in 0..self.diggers {
                    self.sys.readdir(i as usize);
                }

                while !self.alldead
                    && !self.gamedat[self.curplayer as usize].levdone
                    && !self.sys.quit
                    && !self.timeout
                {
                    self.penalty = 0;
                    self.dodigger();
                    self.domonsters();
                    self.dobags();
                    if self.penalty > 8 {
                        self.incmont(self.penalty - 8);
                    }
                    self.testpause();
                    self.checklevdone();
                }

                self.erasediggers();
                self.sys.music_off();
                let mut t = 20;
                while (self.getnmovingbags() != 0 || t != 0) && !self.sys.quit && !self.timeout {
                    if t != 0 {
                        t -= 1;
                    }
                    self.penalty = 0;
                    self.dobags();
                    self.dodigger();
                    self.domonsters();
                    if self.penalty < 8 {
                        t = 0;
                    }
                }
                self.sys.sound_stop();
                for i in 0..self.diggers {
                    // `killfire` uses a global digger slot, so offset by the
                    // active player in alternating mode.
                    self.killfire((self.curplayer + i) as usize);
                }
                self.erasebonus();
                self.cleanupbags();
                self.savefield();
                self.erasemonsters();

                if self.gamedat[self.curplayer as usize].levdone {
                    self.sys.sound_level_done();
                    self.sys.wait_level_done();
                }

                if self.countem() == 0 || self.gamedat[self.curplayer as usize].levdone {
                    for i in self.curplayer..self.diggers + self.curplayer {
                        if self.getlives(i) > 0 && !self.digalive(i) {
                            self.declife(i);
                        }
                    }
                    self.drawlives();
                    self.gamedat[self.curplayer as usize].level += 1;
                    if self.gamedat[self.curplayer as usize].level > 1000 {
                        self.gamedat[self.curplayer as usize].level = 1000;
                    }
                    self.initlevel();
                } else if self.alldead {
                    for i in self.curplayer..self.curplayer + self.diggers {
                        if self.getlives(i) > 0 {
                            self.declife(i);
                        }
                    }
                    self.drawlives();
                }

                if (self.alldead && self.getalllives() == 0 && !self.gauntlet && !self.sys.quit)
                    || self.timeout
                {
                    self.endofgame();
                }
            }
            self.alldead = false;
            if self.nplayers == 2 && self.getlives(1 - self.curplayer) != 0 {
                self.curplayer = 1 - self.curplayer;
                flashplayer = true;
                self.levnotdrawn = true;
            }
        }
        self.save_scores();
    }

    // ------------------------------------------------------------------
    // Level setup / screens
    // ------------------------------------------------------------------

    fn initlevel(&mut self) {
        self.gamedat[self.curplayer as usize].levdone = false;
        self.makefield();
        self.makeemfield();
        self.initbags();
        self.levnotdrawn = true;
    }

    fn drawscreen(&mut self) {
        self.creatembspr();
        self.drawstatics();
        self.drawbags();
        self.drawemeralds();
        self.initdigger();
        self.initmonsters();
    }

    fn initchars(&mut self) {
        self.initmbspr();
        self.initdigger();
        self.initmonsters();
    }

    fn drawstatics(&mut self) {
        for i in 0..MSIZE {
            self.field[i] = if self.curplayer == 0 {
                self.field1[i]
            } else {
                self.field2[i]
            };
        }
        self.sys.screen.set_palette(0);
        self.sys.screen.set_intensity(0);
        self.sys.screen.draw_background(self.levplan());
        self.sys.screen.draw_field(&mut self.field);
    }

    fn makefield(&mut self) {
        for x in 0..MWIDTH {
            for y in 0..MHEIGHT {
                let idx = y * MWIDTH + x;
                let mut f: u16 = 0xffff;
                let c = self.getlevch(x as i32, y as i32, self.levplan());
                if c == b'S' || c == b'V' {
                    f &= 0xd03f;
                }
                if c == b'S' || c == b'H' {
                    f &= 0xdfe0;
                }
                self.field[idx] = f;
                if self.curplayer == 0 {
                    self.field1[idx] = f;
                } else {
                    self.field2[idx] = f;
                }
            }
        }
    }

    fn savefield(&mut self) {
        for i in 0..MSIZE {
            if self.curplayer == 0 {
                self.field1[i] = self.field[i];
            } else {
                self.field2[i] = self.field[i];
            }
        }
    }

    fn getlevch(&self, x: i32, y: i32, l: i32) -> u8 {
        if (l == 3 || l == 4) && self.diggers == 2 && y == 9 && (x == 6 || x == 8) {
            return b'H';
        }
        let l = (l.clamp(1, 8) - 1) as usize;
        LEVELS[l][y as usize][x as usize]
    }

    fn checklevdone(&mut self) {
        let done = (self.countem() == 0 || self.monleft() == 0) && self.isalive();
        self.gamedat[self.curplayer as usize].levdone = done;
    }

    fn incpenalty(&mut self) {
        self.penalty += 1;
    }

    fn levplan(&self) -> i32 {
        crate::levels::level_plan(self.levno())
    }

    fn levof10(&self) -> i32 {
        self.gamedat[self.curplayer as usize].level.min(10)
    }

    pub fn levno(&self) -> i32 {
        self.gamedat[self.curplayer as usize].level
    }

    fn cleartopline(&mut self) {
        self.outtext("                          ", 0, 0, 3);
        self.outtext(" ", 308, 0, 3);
    }

    fn testpause(&mut self) {
        if self.sys.input.pausef {
            self.sys.input.pausef = false;
            self.sys.sound_pause();
            self.cleartopline();
            self.outtext("PRESS ANY KEY", 80, 0, 1);
            self.sys.wait_key();
            self.cleartopline();
            self.drawscores();
            for i in 0..self.diggers {
                self.addscore(i as usize, 0);
            }
            self.drawlives();
        } else {
            self.sys.sound_pause_off();
        }
    }

    fn shownplayers(&mut self) {
        // Clear the previous mode text (the original Windows build does this
        // via clear_nplayerlines()).
        self.outtext("             ", 168, 25, 3);
        self.outtext("             ", 168, 39, 3);
        if self.diggers == 2 {
            if self.gauntlet {
                self.outtext("TWO PLAYER", 180, 25, 3);
                self.outtext("GAUNTLET", 192, 39, 3);
            } else {
                self.outtext("TWO PLAYER", 180, 25, 3);
                self.outtext("SIMULTANEOUS", 170, 39, 3);
            }
        } else if self.gauntlet {
            self.outtext("GAUNTLET", 192, 25, 3);
            self.outtext("MODE", 216, 39, 3);
        } else if self.nplayers == 1 {
            self.outtext("ONE", 220, 25, 3);
            self.outtext(" PLAYER ", 192, 39, 3);
        } else {
            self.outtext("TWO", 220, 25, 3);
            self.outtext(" PLAYERS", 184, 39, 3);
        }
    }

    fn showtable(&mut self) {
        self.outtext("HIGH SCORES", 16, 25, 3);
        for i in 0..10 {
            let mut s = String::new();
            for &b in &self.highscores[i].name {
                s.push(if b == 0 { '.' } else { b as char });
            }
            s.push(' ');
            let digits = self.highscores[i].score.to_string();
            for _ in digits.len()..7 {
                s.push(' ');
            }
            s.push_str(&digits);
            self.outtext(&s, 16, 31 + 13 * (i as i32 + 1), if i == 0 { 2 } else { 1 });
        }
    }

    // ------------------------------------------------------------------
    // Small helpers
    // ------------------------------------------------------------------

    pub fn outtext(&mut self, text: &str, x: i32, y: i32, style: u8) {
        self.sys.screen.outtext(text, x, y, style);
    }

    fn writenum(&mut self, n: i64, x: i32, y: i32, width: i32, style: u8) {
        self.sys.screen.writenum(n, x, y, width, style);
    }

    fn getalllives(&self) -> i32 {
        let mut t = 0;
        for i in self.curplayer..self.curplayer + self.diggers {
            t += self.getlives(i);
        }
        t
    }

    fn seed_rng(&mut self) {
        let seed = if let Some(seed) = self.rng_seed {
            seed
        } else {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(1)
        };
        self.randv = seed.wrapping_mul(1193).max(1);
    }

    /// `randno()` from `main.c`.
    pub fn randno(&mut self, n: i32) -> i32 {
        self.randv = self.randv.wrapping_mul(0x15a4e35).wrapping_add(1);
        ((self.randv & 0x7fff_ffff) % n as i64) as i32
    }

    fn getfield(&self, x: i32, y: i32) -> u16 {
        if x < 0 || y < 0 || x >= MWIDTH as i32 || y >= MHEIGHT as i32 {
            return 0xffff;
        }
        self.field[y as usize * MWIDTH + x as usize]
    }

    fn clfirst(&self) -> [i32; TYPES] {
        self.sys.sprites.first
    }

    fn clcoll(&self) -> [i32; SPRITES] {
        self.sys.sprites.coll
    }

    fn drawgold(&mut self, n: usize, t: i32, x: i32, y: i32) {
        let idx = FIRSTBAG + n;
        self.sys.sprites.initspr(idx, t + 62, 4, 15, 0, 0);
        self.sys.sprites.drawspr(idx, x, y);
    }

    fn drawbonus(&mut self, x: i32, y: i32) {
        self.sys.sprites.initspr(FIRSTBONUS, 81, 4, 15, 0, 0);
        self.sys.sprites.movedrawspr(FIRSTBONUS, x, y);
    }

    pub fn creatembspr(&mut self) {
        for i in 0..BAGS {
            self.sys.sprites.createspr(FIRSTBAG + i, 62, 4, 15, 0, 0);
        }
        for i in 0..MONSTERS {
            self.sys
                .sprites
                .createspr(FIRSTMONSTER + i, 71, 4, 15, 0, 0);
        }
        self.createdbfspr();
        for i in 0..MONSTERS {
            self.monspr[i] = 0;
            self.monspd[i] = 1;
        }
    }

    fn createdbfspr(&mut self) {
        for i in 0..DIGGERS {
            self.digspd[i] = 1;
            self.digspr[i] = 0;
        }
        for i in 0..FIREBALLS {
            self.firespr[i] = 0;
        }
        for i in FIRSTDIGGER..LASTDIGGER {
            self.sys.sprites.createspr(i, 0, 4, 15, 0, 0);
        }
        for i in FIRSTBONUS..LASTBONUS {
            self.sys.sprites.createspr(i, 81, 4, 15, 0, 0);
        }
        for i in FIRSTFIREBALL..LASTFIREBALL {
            self.sys.sprites.createspr(i, 82, 2, 8, 0, 0);
        }
    }

    pub fn initmbspr(&mut self) {
        for i in 0..BAGS {
            self.sys.sprites.initspr(FIRSTBAG + i, 62, 4, 15, 0, 0);
        }
        for i in 0..MONSTERS {
            self.sys.sprites.initspr(FIRSTMONSTER + i, 71, 4, 15, 0, 0);
        }
        self.initdbfspr();
    }

    fn initdbfspr(&mut self) {
        for i in 0..DIGGERS {
            self.digspd[i] = 1;
            self.digspr[i] = 0;
        }
        for i in 0..FIREBALLS {
            self.firespr[i] = 0;
        }
        for i in FIRSTDIGGER..LASTDIGGER {
            self.sys.sprites.initspr(i, 0, 4, 15, 0, 0);
        }
        for i in FIRSTBONUS..LASTBONUS {
            self.sys.sprites.initspr(i, 81, 4, 15, 0, 0);
        }
        for i in FIRSTFIREBALL..LASTFIREBALL {
            self.sys.sprites.initspr(i, 82, 2, 8, 0, 0);
        }
    }

    fn drawlife(&mut self, t: usize, x: i32, y: i32) {
        self.sys.screen.draw_misc(x, y, t + 110, 4, 12);
    }

    fn drawlives(&mut self) {
        if self.gauntlet {
            let g = (self.cgtime / 1_193_181) as i32;
            let s = format!("{:3}:{:02}", g / 60, g % 60);
            self.outtext(&s, 124, 0, 3);
            return;
        }
        let mut n = self.getlives(0) - 1;
        self.outtext("     ", 96, 0, 2);
        if n > 4 {
            self.drawlife(0, 80, 0);
            let s = format!("X{}", n);
            self.outtext(&s, 100, 0, 2);
        } else {
            for l in 1..5 {
                self.drawlife(if n > 0 { 0 } else { 2 }, l * 20 + 60, 0);
                n -= 1;
            }
        }
        if self.nplayers == 2 {
            self.outtext("     ", 164, 0, 2);
            n = self.getlives(1) - 1;
            if n > 4 {
                let s = format!("{}X", n);
                self.outtext(&s, 220 - s.len() as i32 * 12, 0, 2);
                self.drawlife(1, 224, 0);
            } else {
                for l in 1..5 {
                    self.drawlife(if n > 0 { 1 } else { 2 }, 244 - l * 20, 0);
                    n -= 1;
                }
            }
        }
        if self.diggers == 2 {
            self.outtext("     ", 164, 0, 1);
            n = self.getlives(1) - 1;
            if n > 4 {
                let s = format!("{}X", n);
                self.outtext(&s, 220 - s.len() as i32 * 12, 0, 1);
                self.drawlife(3, 224, 0);
            } else {
                for l in 1..5 {
                    self.drawlife(if n > 0 { 3 } else { 2 }, 244 - l * 20, 0);
                    n -= 1;
                }
            }
        }
    }
}
