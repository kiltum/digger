//! The game thread's view of the world: screen, sprites, input, clock, sound.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::assets::Assets;
use crate::constants::*;
use crate::input::{InputState, SharedInput};
use crate::platform::Shared;
use crate::screen::Screen;
use crate::sound::SoundEngine;
use crate::sprites::SpriteSystem;

pub struct Clock {
    /// Frame period in PIT ticks (`ftime` in the original).
    pub ftime: u64,
    next: Instant,
    pub frame: u64,
    /// Headless runs advance without sleeping.
    real_time: bool,
}

impl Clock {
    pub fn new(ftime: u64) -> Self {
        let mut c = Self {
            ftime,
            next: Instant::now(),
            frame: 0,
            real_time: true,
        };
        c.next = Instant::now() + c.tick_duration();
        c
    }

    fn tick_duration(&self) -> Duration {
        Duration::from_secs_f64(self.ftime as f64 / PIT_HZ)
    }

    pub fn adjust_ftime(&mut self, delta: i64) {
        self.ftime = (self.ftime as i64 + delta).max(10_000) as u64;
    }

    /// Block until the next game frame and advance the clock.
    pub fn wait(&mut self) {
        let dur = self.tick_duration();
        if self.real_time {
            let now = Instant::now();
            if self.next > now {
                std::thread::sleep(self.next - now);
            }
            let now = Instant::now();
            self.next += dur;
            if self.next < now {
                self.next = now + dur;
            }
        }
        self.frame += 1;
    }

    pub fn set_real_time(&mut self, real_time: bool) {
        self.real_time = real_time;
    }
}

pub struct Sys {
    pub screen: Screen,
    pub sprites: SpriteSystem,
    pub input: InputState,
    pub shared_input: Arc<SharedInput>,
    pub clock: Clock,
    pub sound: Option<Arc<Mutex<SoundEngine>>>,
    pub shared: Arc<Shared>,
    pub music_enabled: bool,
    pub sound_enabled: bool,
    pub quit: bool,
    pub headless: bool,
    /// Headless runs stop after this many frames.
    pub max_frames: Option<u64>,
    /// Development/test aid: when set via `DIGGER_AUTO_DIR`, player 1 always
    /// reports this direction. `-1` disables it.
    pub debug_dir: i32,
    /// Development/test aid: hold the fire button.
    pub debug_fire: bool,
}

impl Sys {
    pub fn new(
        shared: Arc<Shared>,
        sound: Option<Arc<Mutex<SoundEngine>>>,
    ) -> Result<Self, String> {
        Self::new_with_mode(shared, sound, false, None)
    }

    /// `headless` disables real-time sleeping and can impose a frame limit.
    pub fn new_headless(
        shared: Arc<Shared>,
        sound: Option<Arc<Mutex<SoundEngine>>>,
        max_frames: Option<u64>,
    ) -> Result<Self, String> {
        Self::new_with_mode(shared, sound, true, max_frames)
    }

    fn new_with_mode(
        shared: Arc<Shared>,
        sound: Option<Arc<Mutex<SoundEngine>>>,
        headless: bool,
        max_frames: Option<u64>,
    ) -> Result<Self, String> {
        let assets = Arc::new(Assets::load()?);
        let mut clock = Clock::new(DEFAULT_FTIME);
        clock.set_real_time(!headless);
        Ok(Self {
            screen: Screen::new(assets),
            sprites: SpriteSystem::new(),
            input: InputState::new(),
            shared_input: shared.input.clone(),
            clock,
            sound,
            shared,
            music_enabled: true,
            sound_enabled: true,
            quit: false,
            headless,
            max_frames,
            debug_dir: std::env::var("DIGGER_AUTO_DIR")
                .ok()
                .and_then(|d| match d.to_ascii_lowercase().as_str() {
                    "right" => Some(0),
                    "up" => Some(2),
                    "left" => Some(4),
                    "down" => Some(6),
                    _ => None,
                })
                .unwrap_or(-1),
            debug_fire: std::env::var_os("DIGGER_AUTO_FIRE").is_some(),
        })
    }

    /// One game frame: wait for the clock, poll input, apply global toggles,
    /// compose and publish the screen.
    pub fn newframe(&mut self) {
        self.clock.wait();
        self.input.poll(&self.shared_input);
        if self.input.escape {
            self.quit = true;
        }
        if self.input.toggle_music {
            self.music_enabled = !self.music_enabled;
            let enabled = self.music_enabled;
            self.with_sound(|e| e.set_music_enabled(enabled));
        }
        if self.input.toggle_sound {
            self.sound_enabled = !self.sound_enabled;
            let enabled = self.sound_enabled;
            self.with_sound(|e| e.set_sound_enabled(enabled));
        }
        if self.input.speed_delta != 0 {
            self.clock.adjust_ftime(self.input.speed_delta);
        }
        self.render();
        if let Some(max) = self.max_frames {
            if self.clock.frame >= max {
                self.quit = true;
            }
        }
    }

    fn render(&mut self) {
        self.screen.compose(&self.sprites);
        self.shared.publish(self.screen.frame());
        self.maybe_dump_frame();
    }

    /// Development aid: dump the composed frame as a PPM at `DIGGER_DUMP_AT`.
    /// Enabled by setting `DIGGER_DUMP=/path/frame.ppm` (and optionally
    /// `DIGGER_DUMP_AT=<frame>`).
    fn maybe_dump_frame(&self) {
        let Ok(path) = std::env::var("DIGGER_DUMP") else {
            return;
        };
        let at: u64 = std::env::var("DIGGER_DUMP_AT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8);
        if self.clock.frame != at {
            return;
        }
        let mut out = Vec::with_capacity(FB_W * FB_H * 3 + 16);
        out.extend_from_slice(format!("P6\n{} {}\n255\n", FB_W, FB_H).as_bytes());
        for &px in self.screen.frame() {
            out.push((px >> 16) as u8);
            out.push((px >> 8) as u8);
            out.push(px as u8);
        }
        if let Err(e) = std::fs::write(&path, out) {
            eprintln!("failed to write frame dump: {e}");
        }
        eprintln!("[dump] clock_frame={}", self.clock.frame);
        for (i, r) in self.sprites.recs.iter().enumerate() {
            if r.enabled {
                eprintln!(
                    "[dump] sprite {i}: ch={} x={} y={} w={} h={}",
                    r.ch, r.x, r.y, r.w, r.h
                );
            }
        }
    }

    pub fn readdir(&mut self, n: usize) {
        self.input.readdir(&self.shared_input, n);
    }

    /// Test/automation hooks.
    pub fn set_debug_dir(&mut self, dir: i32) {
        self.debug_dir = dir;
    }

    pub fn set_debug_fire(&mut self, fire: bool) {
        self.debug_fire = fire;
    }

    pub fn getdir(&self, n: usize) -> i32 {
        if n == 0 && self.debug_dir != -1 {
            return self.debug_dir;
        }
        self.input.getdir(n)
    }

    pub fn getfire(&self, n: usize) -> bool {
        if n == 0 && self.debug_fire {
            return true;
        }
        self.input.getfire(n)
    }

    pub fn clearfire(&mut self, n: usize) {
        self.input.clearfire(n);
    }

    /// The original flushes buffered keystrokes at the start of a level; our
    /// input poll already drains the queue once per tick.
    pub fn flush_input_edges(&mut self) {}

    pub fn wait_key(&mut self) -> Option<char> {
        loop {
            self.newframe();
            if self.quit {
                return None;
            }
            if let Some(c) = self.input.chars.first().copied() {
                return Some(c);
            }
            if self.input.any_key {
                return Some('\0');
            }
        }
    }

    pub fn take_char(&mut self) -> Option<char> {
        if self.input.chars.is_empty() {
            None
        } else {
            Some(self.input.chars.remove(0))
        }
    }

    // ------------------------------------------------------------------
    // Sound commands
    // ------------------------------------------------------------------

    fn with_sound<R>(&self, f: impl FnOnce(&mut SoundEngine) -> R) -> Option<R> {
        self.sound.as_ref().map(|m| {
            let mut engine = m.lock().unwrap_or_else(|p| p.into_inner());
            f(&mut engine)
        })
    }

    pub fn sound_stop(&self) {
        let _ = self.with_sound(|e| e.sound_stop());
    }
    pub fn music(&self, tune: i32) {
        // Always record the tune; the engine's musicflag gates playback, just
        // like the original's musicupdate().
        let _ = self.with_sound(|e| e.music(tune));
    }
    pub fn music_off(&self) {
        let _ = self.with_sound(|e| e.music_off());
    }
    pub fn sound_level_done(&self) {
        let _ = self.with_sound(|e| e.level_done_start());
    }
    pub fn wait_level_done(&mut self) {
        if self.headless {
            return;
        }
        loop {
            let active = self.with_sound(|e| e.level_done_active()).unwrap_or(false);
            if !active || self.quit {
                break;
            }
            self.newframe();
        }
    }
    pub fn sound_one_up(&self) {
        let _ = self.with_sound(|e| e.one_up());
    }
    pub fn sound_pause(&self) {
        let _ = self.with_sound(|e| e.pause());
    }
    pub fn sound_pause_off(&self) {
        let _ = self.with_sound(|e| e.pause_off());
    }
    pub fn sound_bonus(&self) {
        let _ = self.with_sound(|e| e.bonus());
    }
    pub fn sound_bonus_off(&self) {
        let _ = self.with_sound(|e| e.bonus_off());
    }
    pub fn sound_fire(&self, n: usize) {
        let _ = self.with_sound(|e| e.fire(n));
    }
    pub fn sound_fire_off(&self, n: usize) {
        let _ = self.with_sound(|e| e.fire_off(n));
    }
    pub fn sound_explode(&self, n: usize) {
        let _ = self.with_sound(|e| e.explode(n));
    }
    pub fn sound_em(&self) {
        let _ = self.with_sound(|e| e.em());
    }
    pub fn sound_emerald(&self, n: usize) {
        let _ = self.with_sound(|e| e.emerald(n));
    }
    pub fn sound_gold(&self) {
        let _ = self.with_sound(|e| e.gold());
    }
    pub fn sound_eat_monster(&self) {
        let _ = self.with_sound(|e| e.eat_monster());
    }
    pub fn sound_digger_die(&self) {
        let _ = self.with_sound(|e| e.digger_die());
    }
    pub fn sound_wobble(&self) {
        let _ = self.with_sound(|e| e.wobble());
    }
    pub fn sound_wobble_off(&self) {
        let _ = self.with_sound(|e| e.wobble_off());
    }
    pub fn sound_fall(&self) {
        let _ = self.with_sound(|e| e.fall());
    }
    pub fn sound_fall_off(&self) {
        let _ = self.with_sound(|e| e.fall_off());
    }
    pub fn sound_break(&self) {
        let _ = self.with_sound(|e| e.break_bag());
    }
}
