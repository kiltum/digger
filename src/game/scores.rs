//! Port of `scores.c`: scoring, extra lives, high-score table and game over.

use std::io::{Read, Write};
use std::path::PathBuf;

use crate::constants::*;
use crate::game::{Game, HighScore};

const SCORE_FILE: &str = "digger.sco";

pub fn load_highscores() -> [HighScore; 10] {
    let mut table = [HighScore {
        name: [b'.'; 3],
        score: 0,
    }; 10];
    let mut text = String::new();
    if let Ok(mut f) = std::fs::File::open(score_file()) {
        if f.read_to_string(&mut text).is_ok() {
            let mut entries: Vec<HighScore> = text
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    let (name, score) = line.split_once(char::is_whitespace)?;
                    let mut n = [b'.'; 3];
                    for (i, b) in name.bytes().take(3).enumerate() {
                        n[i] = b.to_ascii_uppercase();
                    }
                    let score = score.trim().parse::<i64>().ok()?;
                    Some(HighScore { name: n, score })
                })
                .collect();
            entries.sort_by(|a, b| b.score.cmp(&a.score));
            for (dst, src) in table.iter_mut().zip(entries.into_iter()) {
                *dst = src;
            }
        }
    }
    table
}

pub fn save_highscores(table: &[HighScore; 10]) {
    let mut text = String::new();
    for h in table {
        let name: String = h
            .name
            .iter()
            .map(|&b| if b == 0 { '.' } else { b as char })
            .collect();
        text.push_str(&format!("{} {}\n", name, h.score));
    }
    if let Ok(mut f) = std::fs::File::create(score_file()) {
        let _ = f.write_all(text.as_bytes());
    }
}

fn score_file() -> PathBuf {
    // Keep the original "file next to the program" spirit.  A future config
    // layer can move this to the platform data directory.
    if let Ok(dir) = std::env::var("DIGGER_DATA_DIR") {
        return PathBuf::from(dir).join(SCORE_FILE);
    }
    PathBuf::from(SCORE_FILE)
}

impl Game {
    pub fn zeroscores(&mut self) {
        self.scdat[0] = crate::game::ScoreDat {
            score: 0,
            nextbs: BONUS_SCORE,
        };
        self.scdat[1] = crate::game::ScoreDat {
            score: 0,
            nextbs: BONUS_SCORE,
        };
        self.scoret = 0;
    }

    pub(crate) fn writecurscore(&mut self, col: u8) {
        if self.curplayer == 0 {
            let s = self.scdat[0].score;
            self.writenum(s, 0, 0, 6, col);
        } else if self.scdat[1].score < 100_000 {
            let s = self.scdat[1].score;
            self.writenum(s, 236, 0, 6, col);
        } else {
            let s = self.scdat[1].score;
            self.writenum(s, 248, 0, 6, col);
        }
    }

    pub fn drawscores(&mut self) {
        let s0 = self.scdat[0].score;
        self.writenum(s0, 0, 0, 6, 3);
        if self.nplayers == 2 || self.diggers == 2 {
            if self.scdat[1].score < 100_000 {
                let s1 = self.scdat[1].score;
                self.writenum(s1, 236, 0, 6, 3);
            } else {
                let s1 = self.scdat[1].score;
                self.writenum(s1, 248, 0, 6, 3);
            }
        }
    }

    pub fn initscores(&mut self) {
        for i in 0..self.diggers {
            self.addscore(i as usize, 0);
        }
    }

    pub fn addscore(&mut self, n: usize, score: i64) {
        self.scdat[n].score += score;
        if self.scdat[n].score > 999_999 {
            self.scdat[n].score = 0;
        }
        if n == 0 {
            let s = self.scdat[n].score;
            self.writenum(s, 0, 0, 6, 1);
        } else if self.scdat[n].score < 100_000 {
            let s = self.scdat[n].score;
            self.writenum(s, 236, 0, 6, 1);
        } else {
            let s = self.scdat[n].score;
            self.writenum(s, 248, 0, 6, 1);
        }
        if self.scdat[n].score >= self.scdat[n].nextbs + n as i64 && self.scdat[n].score < 1_000_000
        {
            if self.getlives(n as i32) < 5 || self.unlimlives {
                if self.gauntlet {
                    self.cgtime += 17_897_715;
                } else {
                    self.addlife(n as i32);
                }
                self.drawlives();
            }
            self.scdat[n].nextbs += BONUS_SCORE;
        }
        self.incpenalty();
        self.incpenalty();
        self.incpenalty();
    }

    pub fn scorekill(&mut self, n: usize) {
        self.addscore(n, 250);
    }
    pub fn scorekill2(&mut self) {
        self.addscore(0, 125);
        self.addscore(1, 125);
    }
    pub fn scoreemerald(&mut self, n: usize) {
        self.addscore(n, 25);
    }
    pub fn scoreoctave(&mut self, n: usize) {
        self.addscore(n, 250);
    }
    pub fn scoregold(&mut self, n: usize) {
        self.addscore(n, 500);
    }
    pub fn scorebonus(&mut self, n: usize) {
        self.addscore(n, 1000);
    }
    pub fn scoreeatm(&mut self, n: usize, msc: i32) {
        self.addscore(n, msc as i64 * 200);
    }

    /// `endofgame()` — high-score entry and game-over banner.
    pub fn endofgame(&mut self) {
        if self.gauntlet {
            self.cleartopline();
            self.outtext("TIME UP", 120, 0, 3);
            for _ in 0..50 {
                if self.sys.quit {
                    break;
                }
                self.sys.newframe();
            }
            self.outtext("       ", 120, 0, 3);
        }

        let mut any_high = false;
        for i in self.curplayer..self.curplayer + self.diggers {
            self.scoret = self.scdat[i as usize].score;
            let lowest = self.highscores[9].score;
            let qualified = self.scoret > lowest || self.highscores.iter().any(|h| h.score == 0);
            if qualified && self.scoret > 0 {
                any_high = true;
                self.sys.screen.clear();
                self.drawscores();
                let who = if i == 0 { "PLAYER 1" } else { "PLAYER 2" };
                self.outtext(who, 108, 0, 2);
                self.outtext(" NEW HIGH SCORE ", 64, 40, 2);
                self.getinitials();
            }
        }
        // If a new high score was entered, the outer title loop redraws the
        // title screen (as in the original).  Otherwise show GAME OVER.
        if !any_high {
            self.cleartopline();
            self.outtext("GAME OVER", 104, 0, 3);
            for _ in 0..50 {
                if self.sys.quit {
                    break;
                }
                self.sys.newframe();
            }
            self.outtext("         ", 104, 0, 3);
        }
    }

    fn getinitials(&mut self) {
        if self.sys.headless {
            self.insert_highscore([b'C', b'P', b'U'], self.scoret);
            return;
        }
        self.sys.newframe();
        self.outtext("ENTER YOUR", 100, 70, 3);
        self.outtext(" INITIALS", 100, 90, 3);
        self.outtext("_ _ _", 128, 130, 3);
        let mut name = [b'.'; 3];
        for i in 0..3usize {
            let ch = loop {
                self.sys.newframe();
                if self.sys.quit {
                    return;
                }
                if let Some(c) = self.sys.take_char() {
                    if c.is_ascii_alphanumeric() {
                        break c.to_ascii_uppercase();
                    }
                }
            };
            self.sys.screen.gwrite(i as i32 * 24 + 128, 130, ch, 3);
            name[i] = ch as u8;
        }
        self.insert_highscore(name, self.scoret);
    }

    fn insert_highscore(&mut self, name: [u8; 3], score: i64) {
        let mut entries: Vec<HighScore> = self.highscores.to_vec();
        entries.push(HighScore { name, score });
        entries.sort_by(|a, b| b.score.cmp(&a.score));
        entries.truncate(10);
        self.highscores.copy_from_slice(&entries);
        self.save_scores();
    }

    pub fn save_scores(&self) {
        // Headless runs are tests/automation and must not write into the
        // working directory.
        if !self.sys.headless {
            save_highscores(&self.highscores);
        }
    }
}
