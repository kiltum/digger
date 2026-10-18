//! Faithful Rust port of Digger Remastered's audio subsystem.
//!
//! This module is a dependency-free, `unsafe`-free transcription of two C
//! files from Digger Remastered:
//!
//! * `newsnd.c` — the accurate PC-speaker / 8253 PIT emulator
//!   (`getsample1`, `soundinitglob`, the `s1*` timer hooks, `addcarry` /
//!   `subcarry`, and the `lut` / `pwlut` construction).
//! * `sound.c` — the high-level sound state machine (`soundint`, `initsound`,
//!   `sett0`, `setsoundt2`, `setsoundmode`, `startint8`, all effects, and the
//!   complete music tables).
//!
//! The emulator averages the speaker on-time over each output sample.  Timer 0
//! is reprogrammed to the current note period, while the `timercount` /
//! `timerrate` accumulator is used to derive the sound-update tick.  When
//! `timercount` overflows, `soundint()` runs, which advances the music and every
//! sound effect.  With the default 0x4000 timer period that happens at
//! 1,193,181 / 16,384 ≈ 72.8 Hz, matching the comment in the original source
//! (and `routines.txt`'s "sound unit" of 16,384 ticks).
//!
//! Times in the emulator are 16-bit, exactly as in the C build where `Uint4`
//! is `unsigned short`.  In particular `timercount` *must* wrap at 16 bits:
//! after an overflow the C code subtracts 0x4000 (rather than resetting to
//! zero), which relies on modulo-65536 arithmetic to keep the 72.8 Hz cadence.
//!
//! Deliberate adaptations (all forced by the standalone, non-blocking API):
//!
//! * `soundlevdone()`'s blocking `while` loop is replaced by
//!   [`SoundEngine::level_done_start`] / [`SoundEngine::level_done_active`];
//!   `soundlevdoneupdate()` is invoked from the emulator's sound tick.
//! * `fillbuffer`, `initint8`, `restoreint8`, the `s0*` and `s2*` platform
//!   hooks, and the circular sample buffer are no-ops; samples are pulled on
//!   demand by [`SoundEngine::render`].
//! * `getlrt()` (the low-resolution timer used to seed the sound PRNG) is
//!   replaced by a fixed seed so the engine is deterministic.  `randnos()`
//!   keeps the original build's 32-bit `long` arithmetic and wrapping
//!   multiply, so the random fire frequencies are unchanged.
//! * Disabling sound applies the C "sound off" transition immediately instead
//!   of waiting for the next 72.8 Hz tick, so callers see silence at once.
//!
//! The emulator's 8-bit output has value 0 for a fully-off speaker.  It is
//! scaled by [`MASTER_GAIN`] into f32; silence is therefore exactly `0.0`.

/// Minimum sample value produced by the PC-speaker emulator (speaker off).
pub const MIN_SAMP: u8 = 0;
/// Maximum sample value produced by the PC-speaker emulator (speaker on).
pub const MAX_SAMP: u8 = 0xff;

/// Master output gain applied when converting 8-bit samples to f32.
pub const MASTER_GAIN: f32 = 0.35;

/// PIT input clock: 1,193,181 Hz (0x1234dd).
const TIMER_CLOCK_HZ: u32 = 0x0012_34dd;

/// Number of fireball sound channels (C `FIREBALLS` = `DIGGERS` = 2).
const FIREBALLS: usize = 2;

// ---------------------------------------------------------------------------
// Music and jingle data (transcribed verbatim from sound.c)
// ---------------------------------------------------------------------------

/// `newlevjingle[11]` — level-completion fanfare, played through timer 2.
const NEWLEVJINGLE: [i32; 11] = [
    0x8e8, 0x712, 0x5f2, 0x7f0, 0x6ac, 0x54c, 0x712, 0x5f2, 0x4b8, 0x474, 0x474,
];

/// `emfreqs[8]` — emerald pickup frequencies, selected by emerald number.
const EMFREQS: [i32; 8] = [0x8e8, 0x7f0, 0x712, 0x6ac, 0x5f2, 0x54c, 0x4b8, 0x474];

/// `bonusjingle[321]` — tune 0.
const BONUSJINGLE: [i32; 321] = [
    0x11d1, 0x2, 0x11d1, 0x2, 0x11d1, 0x4, 0x11d1, 0x2, 0x11d1, 0x2, 0x11d1, 0x4, 0x11d1, 0x2,
    0x11d1, 0x2, 0xd59, 0x4, 0xbe4, 0x4, 0xa98, 0x4, 0x11d1, 0x2, 0x11d1, 0x2, 0x11d1, 0x4, 0x11d1,
    0x2, 0x11d1, 0x2, 0x11d1, 0x4, 0xd59, 0x2, 0xa98, 0x2, 0xbe4, 0x4, 0xe24, 0x4, 0x11d1, 0x4,
    0x11d1, 0x2, 0x11d1, 0x2, 0x11d1, 0x4, 0x11d1, 0x2, 0x11d1, 0x2, 0x11d1, 0x4, 0x11d1, 0x2,
    0x11d1, 0x2, 0xd59, 0x4, 0xbe4, 0x4, 0xa98, 0x4, 0xd59, 0x2, 0xa98, 0x2, 0x8e8, 0xa, 0xa00,
    0x2, 0xa98, 0x2, 0xbe4, 0x2, 0xd59, 0x4, 0xa98, 0x4, 0xd59, 0x4, 0x11d1, 0x2, 0x11d1, 0x2,
    0x11d1, 0x4, 0x11d1, 0x2, 0x11d1, 0x2, 0x11d1, 0x4, 0x11d1, 0x2, 0x11d1, 0x2, 0xd59, 0x4,
    0xbe4, 0x4, 0xa98, 0x4, 0x11d1, 0x2, 0x11d1, 0x2, 0x11d1, 0x4, 0x11d1, 0x2, 0x11d1, 0x2,
    0x11d1, 0x4, 0xd59, 0x2, 0xa98, 0x2, 0xbe4, 0x4, 0xe24, 0x4, 0x11d1, 0x4, 0x11d1, 0x2, 0x11d1,
    0x2, 0x11d1, 0x4, 0x11d1, 0x2, 0x11d1, 0x2, 0x11d1, 0x4, 0x11d1, 0x2, 0x11d1, 0x2, 0xd59, 0x4,
    0xbe4, 0x4, 0xa98, 0x4, 0xd59, 0x2, 0xa98, 0x2, 0x8e8, 0xa, 0xa00, 0x2, 0xa98, 0x2, 0xbe4, 0x2,
    0xd59, 0x4, 0xa98, 0x4, 0xd59, 0x4, 0xa98, 0x2, 0xa98, 0x2, 0xa98, 0x4, 0xa98, 0x2, 0xa98, 0x2,
    0xa98, 0x4, 0xa98, 0x2, 0xa98, 0x2, 0xa98, 0x4, 0x7f0, 0x4, 0xa98, 0x4, 0x7f0, 0x4, 0xa98, 0x4,
    0x7f0, 0x4, 0xa98, 0x4, 0xbe4, 0x4, 0xd59, 0x4, 0xe24, 0x4, 0xfdf, 0x4, 0xa98, 0x2, 0xa98, 0x2,
    0xa98, 0x4, 0xa98, 0x2, 0xa98, 0x2, 0xa98, 0x4, 0xa98, 0x2, 0xa98, 0x2, 0xa98, 0x4, 0x7f0, 0x4,
    0xa98, 0x4, 0x7f0, 0x4, 0xa98, 0x4, 0x7f0, 0x4, 0x8e8, 0x4, 0x970, 0x4, 0x8e8, 0x4, 0x970, 0x4,
    0x8e8, 0x4, 0xa98, 0x2, 0xa98, 0x2, 0xa98, 0x4, 0xa98, 0x2, 0xa98, 0x2, 0xa98, 0x4, 0xa98, 0x2,
    0xa98, 0x2, 0xa98, 0x4, 0x7f0, 0x4, 0xa98, 0x4, 0x7f0, 0x4, 0xa98, 0x4, 0x7f0, 0x4, 0xa98, 0x4,
    0xbe4, 0x4, 0xd59, 0x4, 0xe24, 0x4, 0xfdf, 0x4, 0xa98, 0x2, 0xa98, 0x2, 0xa98, 0x4, 0xa98, 0x2,
    0xa98, 0x2, 0xa98, 0x4, 0xa98, 0x2, 0xa98, 0x2, 0xa98, 0x4, 0x7f0, 0x4, 0xa98, 0x4, 0x7f0, 0x4,
    0xa98, 0x4, 0x7f0, 0x4, 0x8e8, 0x4, 0x970, 0x4, 0x8e8, 0x4, 0x970, 0x4, 0x8e8, 0x4, 0x7d64,
];

/// `backgjingle[291]` — tune 1 (background music).
const BACKGJINGLE: [i32; 291] = [
    0xfdf, 0x2, 0x11d1, 0x2, 0xfdf, 0x2, 0x1530, 0x2, 0x1ab2, 0x2, 0x1530, 0x2, 0x1fbf, 0x4, 0xfdf,
    0x2, 0x11d1, 0x2, 0xfdf, 0x2, 0x1530, 0x2, 0x1ab2, 0x2, 0x1530, 0x2, 0x1fbf, 0x4, 0xfdf, 0x2,
    0xe24, 0x2, 0xd59, 0x2, 0xe24, 0x2, 0xd59, 0x2, 0xfdf, 0x2, 0xe24, 0x2, 0xfdf, 0x2, 0xe24, 0x2,
    0x11d1, 0x2, 0xfdf, 0x2, 0x11d1, 0x2, 0xfdf, 0x2, 0x1400, 0x2, 0xfdf, 0x4, 0xfdf, 0x2, 0x11d1,
    0x2, 0xfdf, 0x2, 0x1530, 0x2, 0x1ab2, 0x2, 0x1530, 0x2, 0x1fbf, 0x4, 0xfdf, 0x2, 0x11d1, 0x2,
    0xfdf, 0x2, 0x1530, 0x2, 0x1ab2, 0x2, 0x1530, 0x2, 0x1fbf, 0x4, 0xfdf, 0x2, 0xe24, 0x2, 0xd59,
    0x2, 0xe24, 0x2, 0xd59, 0x2, 0xfdf, 0x2, 0xe24, 0x2, 0xfdf, 0x2, 0xe24, 0x2, 0x11d1, 0x2,
    0xfdf, 0x2, 0x11d1, 0x2, 0xfdf, 0x2, 0xe24, 0x2, 0xd59, 0x4, 0xa98, 0x2, 0xbe4, 0x2, 0xa98,
    0x2, 0xd59, 0x2, 0x11d1, 0x2, 0xd59, 0x2, 0x1530, 0x4, 0xa98, 0x2, 0xbe4, 0x2, 0xa98, 0x2,
    0xd59, 0x2, 0x11d1, 0x2, 0xd59, 0x2, 0x1530, 0x4, 0xa98, 0x2, 0x970, 0x2, 0x8e8, 0x2, 0x970,
    0x2, 0x8e8, 0x2, 0xa98, 0x2, 0x970, 0x2, 0xa98, 0x2, 0x970, 0x2, 0xbe4, 0x2, 0xa98, 0x2, 0xbe4,
    0x2, 0xa98, 0x2, 0xd59, 0x2, 0xa98, 0x4, 0xa98, 0x2, 0xbe4, 0x2, 0xa98, 0x2, 0xd59, 0x2,
    0x11d1, 0x2, 0xd59, 0x2, 0x1530, 0x4, 0xa98, 0x2, 0xbe4, 0x2, 0xa98, 0x2, 0xd59, 0x2, 0x11d1,
    0x2, 0xd59, 0x2, 0x1530, 0x4, 0xa98, 0x2, 0x970, 0x2, 0x8e8, 0x2, 0x970, 0x2, 0x8e8, 0x2,
    0xa98, 0x2, 0x970, 0x2, 0xa98, 0x2, 0x970, 0x2, 0xbe4, 0x2, 0xa98, 0x2, 0xbe4, 0x2, 0xa98, 0x2,
    0xd59, 0x2, 0xa98, 0x4, 0x7f0, 0x2, 0x8e8, 0x2, 0xa98, 0x2, 0xd59, 0x2, 0x11d1, 0x2, 0xd59,
    0x2, 0x1530, 0x4, 0xa98, 0x2, 0xbe4, 0x2, 0xa98, 0x2, 0xd59, 0x2, 0x11d1, 0x2, 0xd59, 0x2,
    0x1530, 0x4, 0xa98, 0x2, 0x970, 0x2, 0x8e8, 0x2, 0x970, 0x2, 0x8e8, 0x2, 0xa98, 0x2, 0x970,
    0x2, 0xa98, 0x2, 0x970, 0x2, 0xbe4, 0x2, 0xa98, 0x2, 0xbe4, 0x2, 0xd59, 0x2, 0xbe4, 0x2, 0xa98,
    0x4, 0x7d64,
];

/// `dirge[]` — tune 2 (death dirge).
const DIRGE: [i32; 49] = [
    0x7d00, 0x2, 0x11d1, 0x6, 0x11d1, 0x4, 0x11d1, 0x2, 0x11d1, 0x6, 0xefb, 0x4, 0xfdf, 0x2, 0xfdf,
    0x4, 0x11d1, 0x2, 0x11d1, 0x4, 0x12e0, 0x2, 0x11d1, 0xc, 0x7d00, 0x10, 0x7d00, 0x10, 0x7d00,
    0x10, 0x7d00, 0x10, 0x7d00, 0x10, 0x7d00, 0x10, 0x7d00, 0x10, 0x7d00, 0x10, 0x7d00, 0x10,
    0x7d00, 0x10, 0x7d00, 0x10, 0x7d00, 0x10, 0x7d64,
];

/// Table terminator used by `musicupdate` to loop the tunes.
const TUNE_END: i32 = 0x7d64;

// ---------------------------------------------------------------------------
// 16-bit carry helpers (C `addcarry` / `subcarry`)
// ---------------------------------------------------------------------------

/// Add `add` to `dest`, returning true when the 16-bit counter wrapped.
#[inline]
fn addcarry(dest: &mut u16, add: u16) -> bool {
    *dest = dest.wrapping_add(add);
    *dest < add
}

/// Subtract `sub` from `dest`, returning true when the 16-bit counter wrapped.
///
/// The C test is `*dest >= (Uint4)(-sub)`, with `dest` and `-sub` both reduced
/// modulo 65536.
#[inline]
fn subcarry(dest: &mut u16, sub: u16) -> bool {
    *dest = dest.wrapping_sub(sub);
    *dest >= 0u16.wrapping_sub(sub)
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Digger's complete sound engine: PC-speaker emulator plus game sound state.
pub struct SoundEngine {
    // --- newsnd.c globals ---------------------------------------------------
    /// Output sample rate in Hz.
    sample_rate: u32,
    /// PIT ticks per output sample (`0x1234dd / sample_rate`, at least 1).
    rate: i32,
    /// Timer 0 current value / period (16-bit).
    t0v: u16,
    /// Timer 2 current value (16-bit).
    t2v: u16,
    /// Timer 0 reload value (16-bit).
    t0rate: u16,
    /// Timer 2 current reload value (16-bit).
    t2rate: u16,
    /// Timer 2 next reload value (16-bit).
    t2new: u16,
    /// Remaining int8 pulse length (signed: goes negative when it expires).
    i8pulse: i32,
    /// Internal timer-2 square-wave phase flag.
    t2f: bool,
    /// Speaker is connected to timer 2.
    t2sw: bool,
    /// Timer-0 interrupt flag latched until the tick is serviced.
    i8flag: bool,
    /// On-time -> sample lookup table, entries 0..=rate are filled in.
    lut: [u8; 257],
    /// Pulse-width lookup table (`pwlut[1..=50]`).
    pwlut: [u32; 51],

    // --- sound.c globals ----------------------------------------------------
    wavetype: i32,
    musvol: i32,
    spkrmode: i32,
    timerrate: i32,
    timercount: u16,
    t2val: u16,
    t0val: u16,
    pulsewidth: i32,
    volume: i32,
    /// Sound-update counter; only the removed blocking loop consumed it.
    #[allow(dead_code)]
    timerclock: i8,

    soundflag: bool,
    musicflag: bool,
    sndflag: bool,
    soundpausedflag: bool,
    randvs: i32,

    // level-done jingle
    soundlevdoneflag: bool,
    nljpointer: i32,
    nljnoteduration: i32,

    // falling bag
    soundfallflag: bool,
    soundfallf: bool,
    soundfallvalue: i32,
    soundfalln: i32,

    // bag break
    soundbreakflag: bool,
    soundbreakduration: i32,
    soundbreakvalue: i32,

    // bag wobble
    soundwobbleflag: bool,
    soundwobblen: i32,

    // fire
    soundfireflag: [bool; FIREBALLS],
    sff: [bool; FIREBALLS],
    soundfirevalue: [i32; FIREBALLS],
    soundfiren: [i32; FIREBALLS],
    soundfirew: i32,

    // explosion
    soundexplodeflag: [bool; FIREBALLS],
    sef: [bool; FIREBALLS],
    soundexplodevalue: [i32; FIREBALLS],
    soundexplodeduration: [i32; FIREBALLS],
    soundexplodew: i32,

    // bonus
    soundbonusflag: bool,
    soundbonusn: i32,

    // emerald collected
    soundemflag: bool,

    // emerald
    soundemeraldflag: bool,
    soundemeraldduration: i32,
    emerfreq: i32,
    soundemeraldn: i32,

    // gold bag
    soundgoldflag: bool,
    soundgoldf: bool,
    soundgoldvalue1: i32,
    soundgoldvalue2: i32,
    soundgoldduration: i32,

    // eat monster
    soundeatmflag: bool,
    soundeatmvalue: i32,
    soundeatmduration: i32,
    soundeatmn: i32,

    // digger death
    soundddieflag: bool,
    soundddien: i32,
    soundddievalue: i32,

    // extra life
    sound1upflag: bool,
    sound1upduration: i32,

    // music
    musicplaying: bool,
    musicp: i32,
    tuneno: i32,
    noteduration: i32,
    notevalue: i32,
    musicmaxvol: i32,
    musicattackrate: i32,
    musicsustainlevel: i32,
    musicdecayrate: i32,
    musicnotewidth: i32,
    musicreleaserate: i32,
    musicstage: i32,
    musicn: i32,

    // speaker mode bookkeeping
    soundt0flag: bool,
    int8flag: bool,

    // Simple one-pole DC blocker.  The PC speaker output is unipolar
    // (0 = off, 255 = fully on); real hardware AC-couples it.  Removing the
    // DC offset keeps the modern output centred and avoids a pop on start.
    dc_x1: f32,
    dc_y1: f32,
}

impl SoundEngine {
    /// Create an engine at `sample_rate` Hz and run the C `soundinitglob` +
    /// `initsound` initialisation sequence.
    pub fn new(sample_rate: u32) -> Self {
        // soundinitglob() clamps the sample rate, then derives the number of
        // PIT ticks represented by one output sample.
        let samprate = sample_rate.max(5000);
        let rate = (TIMER_CLOCK_HZ / samprate).max(1) as i32;

        let mut engine = SoundEngine {
            sample_rate: samprate,
            rate,
            t0v: 0,
            t2v: 0,
            t0rate: 0,
            t2rate: 0,
            t2new: 0,
            i8pulse: 0,
            t2f: false,
            t2sw: false,
            i8flag: false,
            lut: [0; 257],
            pwlut: [0; 51],

            wavetype: 0,
            musvol: 0,
            spkrmode: 0,
            timerrate: 0x7d0,
            timercount: 0,
            t2val: 0,
            t0val: 0,
            pulsewidth: 1,
            // `calibrate()` in the game loop derives this from CPU speed; the
            // emulated/SoundBlaster setup in main.c uses volume = 1.
            volume: 1,
            timerclock: 0,

            soundflag: true,
            musicflag: true,
            sndflag: false,
            soundpausedflag: false,
            randvs: 0,

            soundlevdoneflag: false,
            nljpointer: 0,
            nljnoteduration: 0,

            soundfallflag: false,
            soundfallf: false,
            soundfallvalue: 0,
            soundfalln: 0,

            soundbreakflag: false,
            soundbreakduration: 0,
            soundbreakvalue: 0,

            soundwobbleflag: false,
            soundwobblen: 0,

            soundfireflag: [false; FIREBALLS],
            sff: [false; FIREBALLS],
            soundfirevalue: [0; FIREBALLS],
            soundfiren: [0; FIREBALLS],
            soundfirew: 0,

            soundexplodeflag: [false; FIREBALLS],
            sef: [false; FIREBALLS],
            soundexplodevalue: [0; FIREBALLS],
            soundexplodeduration: [0; FIREBALLS],
            soundexplodew: 0,

            soundbonusflag: false,
            soundbonusn: 0,

            soundemflag: false,

            soundemeraldflag: false,
            soundemeraldduration: 0,
            emerfreq: 0,
            soundemeraldn: 0,

            soundgoldflag: false,
            soundgoldf: false,
            soundgoldvalue1: 0,
            soundgoldvalue2: 0,
            soundgoldduration: 0,

            soundeatmflag: false,
            soundeatmvalue: 0,
            soundeatmduration: 0,
            soundeatmn: 0,

            soundddieflag: false,
            soundddien: 0,
            soundddievalue: 0,

            sound1upflag: false,
            sound1upduration: 0,

            musicplaying: false,
            musicp: 0,
            tuneno: 0,
            noteduration: 0,
            notevalue: 0,
            musicmaxvol: 0,
            musicattackrate: 0,
            musicsustainlevel: 0,
            musicdecayrate: 0,
            musicnotewidth: 0,
            musicreleaserate: 0,
            musicstage: 0,
            musicn: 0,

            soundt0flag: false,
            int8flag: false,
            dc_x1: 0.0,
            dc_y1: 0.0,
        };

        // soundinitglob(): build the on-time and pulse-width tables.
        engine.t2sw = false;
        let rate_u = rate as usize;
        for i in 0..=rate_u {
            engine.lut[i] = (((i * (MAX_SAMP as usize - MIN_SAMP as usize)) / rate_u) as u8)
                .saturating_add(MIN_SAMP);
        }
        for i in 1..=50 {
            engine.pwlut[i] = (16 + i as u32 * 18) >> 2;
        }

        engine.initsound();
        engine
    }

    /// Configured output sample rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    // -- public game-facing API ---------------------------------------------

    /// Global sound on/off switch (C `soundflag`, toggled by the sound key).
    ///
    /// Turning sound off applies the C "sound off" transition immediately so
    /// that rendering is silent at once rather than at the next 72.8 Hz tick.
    pub fn set_sound_enabled(&mut self, on: bool) {
        self.soundflag = on;
        if !on && self.sndflag {
            self.sndflag = false;
            self.s1timer2(40);
            self.setsoundt2();
            self.s1soundoff();
        }
    }

    /// Music on/off switch (C `musicflag`).
    pub fn set_music_enabled(&mut self, on: bool) {
        self.musicflag = on;
    }

    /// Stop every effect and the music (C `soundstop`).
    pub fn sound_stop(&mut self) {
        self.soundfalloff();
        self.soundwobbleoff();
        for n in 0..FIREBALLS {
            self.soundfireoff(n);
        }
        self.musicoff();
        self.soundbonusoff();
        for n in 0..FIREBALLS {
            self.soundexplodeoff(n);
        }
        self.soundbreakoff();
        self.soundemoff();
        self.soundemeraldoff();
        self.soundgoldoff();
        self.soundeatmoff();
        self.soundddieoff();
        self.sound1upoff();
    }

    /// Start a tune: 0 = bonus jingle, 1 = background, 2 = dirge.
    pub fn music(&mut self, tune: i32) {
        self.tuneno = tune;
        self.musicp = 0;
        self.noteduration = 0;
        match tune {
            0 => {
                self.musicmaxvol = 50;
                self.musicattackrate = 20;
                self.musicsustainlevel = 20;
                self.musicdecayrate = 10;
                self.musicreleaserate = 4;
            }
            1 => {
                self.musicmaxvol = 50;
                self.musicattackrate = 50;
                self.musicsustainlevel = 8;
                self.musicdecayrate = 15;
                self.musicreleaserate = 1;
            }
            2 => {
                self.musicmaxvol = 50;
                self.musicattackrate = 50;
                self.musicsustainlevel = 25;
                self.musicdecayrate = 5;
                self.musicreleaserate = 1;
            }
            _ => {}
        }
        self.musicplaying = true;
        if tune == 2 {
            self.soundddieoff();
        }
    }

    /// Stop the current tune (C `musicoff`).
    pub fn music_off(&mut self) {
        self.musicplaying = false;
        self.musicp = 0;
    }

    /// Begin the level-completion fanfare without blocking.
    ///
    /// This is the non-blocking replacement for C `soundlevdone()`: it sets up
    /// the same state, and `soundlevdoneupdate()` is then advanced once per
    /// emulator sound tick.
    pub fn level_done_start(&mut self) {
        self.sound_stop();
        self.nljpointer = 0;
        self.nljnoteduration = 20;
        self.soundlevdoneflag = true;
        self.soundpausedflag = true;
    }

    /// True while the level-completion jingle is still playing.
    pub fn level_done_active(&self) -> bool {
        self.soundlevdoneflag
    }

    /// Extra-life jingle.
    pub fn one_up(&mut self) {
        self.sound1upduration = 96;
        self.sound1upflag = true;
    }

    /// Pause sound updates (C `soundpause`).
    pub fn pause(&mut self) {
        self.soundpausedflag = true;
    }

    /// Resume sound updates (C `soundpauseoff`).
    pub fn pause_off(&mut self) {
        self.soundpausedflag = false;
    }

    /// Bonus-collected jingle.
    pub fn bonus(&mut self) {
        self.soundbonusflag = true;
    }

    /// Stop the bonus jingle.
    pub fn bonus_off(&mut self) {
        self.soundbonusflag = false;
        self.soundbonusn = 0;
    }

    /// Start fire sound `n`.
    pub fn fire(&mut self, n: usize) {
        if n < FIREBALLS {
            self.soundfirevalue[n] = 500;
            self.soundfireflag[n] = true;
        }
    }

    /// Stop fire sound `n`.
    pub fn fire_off(&mut self, n: usize) {
        if n < FIREBALLS {
            self.soundfireflag[n] = false;
            self.soundfiren[n] = 0;
        }
    }

    /// Start explosion sound `n`.
    pub fn explode(&mut self, n: usize) {
        if n < FIREBALLS {
            self.soundfireoff(n);
            self.soundexplodevalue[n] = 1500;
            self.soundexplodeduration[n] = 10;
            self.soundexplodeflag[n] = true;
        }
    }

    /// Stop explosion sound `n`.
    pub fn explode_off(&mut self, n: usize) {
        if n < FIREBALLS {
            self.soundexplodeflag[n] = false;
        }
    }

    /// "Emerald collected" blip (one-shot).
    pub fn em(&mut self) {
        self.soundemflag = true;
    }

    /// Stop the "emerald collected" blip.
    pub fn em_off(&mut self) {
        self.soundemflag = false;
    }

    /// Start the emerald-collected tone for emerald number `n`.
    pub fn emerald(&mut self, n: usize) {
        self.emerfreq = EMFREQS[n.min(EMFREQS.len() - 1)];
        self.soundemeraldduration = 7;
        self.soundemeraldn = 0;
        self.soundemeraldflag = true;
    }

    /// Stop the emerald tone.
    pub fn emerald_off(&mut self) {
        self.soundemeraldflag = false;
    }

    /// Start the gold-bag jingle.
    pub fn gold(&mut self) {
        self.soundgoldvalue1 = 500;
        self.soundgoldvalue2 = 4000;
        self.soundgoldduration = 30;
        self.soundgoldf = false;
        self.soundgoldflag = true;
    }

    /// Stop the gold-bag jingle.
    pub fn gold_off(&mut self) {
        self.soundgoldflag = false;
    }

    /// Start the "monster eaten" sound.
    pub fn eat_monster(&mut self) {
        self.soundeatmduration = 20;
        self.soundeatmn = 3;
        self.soundeatmvalue = 2000;
        self.soundeatmflag = true;
    }

    /// Stop the "monster eaten" sound.
    pub fn eat_monster_off(&mut self) {
        self.soundeatmflag = false;
    }

    /// Start the digger-death sound.
    pub fn digger_die(&mut self) {
        self.soundddien = 0;
        self.soundddievalue = 20000;
        self.soundddieflag = true;
    }

    /// Stop the digger-death sound.
    pub fn digger_die_off(&mut self) {
        self.soundddieflag = false;
    }

    /// Start the bag-wobble sound.
    pub fn wobble(&mut self) {
        self.soundwobbleflag = true;
    }

    /// Stop the bag-wobble sound.
    pub fn wobble_off(&mut self) {
        self.soundwobbleflag = false;
        self.soundwobblen = 0;
    }

    /// Start the falling-bag sound.
    pub fn fall(&mut self) {
        self.soundfallvalue = 1000;
        self.soundfallflag = true;
    }

    /// Stop the falling-bag sound.
    pub fn fall_off(&mut self) {
        self.soundfallflag = false;
        self.soundfalln = 0;
    }

    /// Bag-break sound (short one-shot).
    pub fn break_bag(&mut self) {
        self.soundbreakduration = 3;
        if self.soundbreakvalue < 15000 {
            self.soundbreakvalue = 15000;
        }
        self.soundbreakflag = true;
    }

    /// Generate mono f32 samples, one per output element.
    pub fn render(&mut self, out: &mut [f32]) {
        for slot in out.iter_mut() {
            let sample = self.getsample1();
            let x = sample as f32 / MAX_SAMP as f32;
            // First-order high-pass: remove the unipolar speaker's DC term.
            let y = x - self.dc_x1 + 0.995 * self.dc_y1;
            self.dc_x1 = x;
            self.dc_y1 = y;
            *slot = (y * MASTER_GAIN).clamp(-1.0, 1.0);
        }
    }

    // -- initialisation ------------------------------------------------------

    /// C `initsound()`.
    fn initsound(&mut self) {
        self.s1settimer2(40);
        self.setspkrt2();
        self.settimer0(0);
        self.wavetype = 2;
        self.t0val = 12000;
        self.musvol = 8;
        self.t2val = 40;
        self.soundt0flag = true;
        self.sndflag = true;
        self.spkrmode = 0;
        self.int8flag = false;
        self.setsoundt2();
        self.sound_stop();
        // setupsound() == s1setupsound(): inittimer (no-op), curtime = 0,
        // startint8(), then the C code primes the sample buffer with one
        // getsample() call.  Keep that call so the emulator phase matches.
        self.startint8();
        let _ = self.getsample1();
        self.timerrate = 0x4000;
        self.settimer0(0x4000);
        // getlrt() is platform-specific; use a fixed seed for determinism.
        self.randvs = 0x1234_5678u32 as i32;
    }

    /// C `startint8()`.
    fn startint8(&mut self) {
        if !self.int8flag {
            // initint8() is a no-op in this port.
            self.timerrate = 0x4000;
            self.settimer0(0x4000);
            self.int8flag = true;
        }
    }

    // -- speaker/timer hooks (the `s1*` family) ------------------------------

    /// C `s1settimer2`: load timer 2, resetting its value.
    fn s1settimer2(&mut self, mut t2: u16) {
        if t2 == 40 {
            t2 = self.rate as u16; // otherwise aliasing causes noise artifacts
        }
        t2 >>= 1;
        self.t2v = t2;
        self.t2new = t2;
    }

    /// C `s1timer2`: change the timer-2 reload without the `s1settimer2` reset.
    fn s1timer2(&mut self, mut t2: u16) {
        if t2 == 40 {
            t2 = self.rate as u16;
        }
        t2 >>= 1;
        self.t2new = t2;
        self.t2rate = t2;
        self.t2v = self.t2rate;
    }

    /// C `s1soundoff`.
    fn s1soundoff(&mut self) {
        self.t2sw = false;
    }

    /// C `s1setspkrt2`.
    fn setspkrt2(&mut self) {
        self.t2sw = true;
    }

    /// C `s1settimer0`.
    fn settimer0(&mut self, t0: u16) {
        self.t0v = t0;
        self.t0rate = t0;
    }

    /// C `s1timer0`.
    fn timer0(&mut self, t0: u16) {
        self.t0rate = t0;
    }

    /// C `timer2()` dispatch (the emulator only has timer 2).
    fn timer2(&mut self, t2: u16) {
        self.s1timer2(t2);
    }

    /// C `sett2val()`.
    fn sett2val(&mut self, t2v: u16) {
        if self.sndflag {
            self.timer2(t2v);
        }
    }

    // -- high-level sound state machine -------------------------------------

    /// C `soundint()`: advance music and all effects at ~72.8 Hz.
    fn soundint(&mut self) {
        self.timerclock = self.timerclock.wrapping_add(1);

        if self.soundflag && !self.sndflag {
            // Turning sound back on always re-enables music, as in the C code.
            self.sndflag = true;
            self.musicflag = true;
        }
        if !self.soundflag && self.sndflag {
            self.sndflag = false;
            self.timer2(40);
            self.setsoundt2();
            self.s1soundoff();
        }

        if self.sndflag && !self.soundpausedflag {
            self.t0val = 0x7d00;
            self.t2val = 40;
            if self.musicflag {
                self.musicupdate();
            }
            self.soundemeraldupdate();
            self.soundwobbleupdate();
            self.soundddieupdate();
            self.soundbreakupdate();
            self.soundgoldupdate();
            self.soundemupdate();
            self.soundexplodeupdate();
            self.soundfireupdate();
            self.soundeatmupdate();
            self.soundfallupdate();
            self.sound1upupdate();
            self.soundbonusupdate();
            if self.t0val == 0x7d00 || self.t2val != 40 {
                self.setsoundt2();
            } else {
                self.setsoundmode();
                self.sett0();
            }
            let t2val = self.t2val;
            self.sett2val(t2val);
        }

        // In the original this runs in soundlevdone()'s blocking loop just
        // after each sound interval; here it is driven by sample generation.
        if self.soundlevdoneflag {
            self.soundlevdoneupdate();
        }
    }

    /// C `sett0()`.
    fn sett0(&mut self) {
        if self.sndflag {
            let t2val = self.t2val;
            self.timer2(t2val);
            if self.t0val < 1000 && (self.wavetype == 1 || self.wavetype == 2) {
                self.t0val = 1000;
            }
            let t0val = self.t0val;
            self.timer0(t0val);
            self.timerrate = t0val as i32;
            if self.musvol < 1 {
                self.musvol = 1;
            }
            if self.musvol > 50 {
                self.musvol = 50;
            }
            self.pulsewidth = self.musvol * self.volume;
            self.setsoundmode();
        }
    }

    /// C `setsoundt2()`.
    fn setsoundt2(&mut self) {
        if self.soundt0flag {
            self.spkrmode = 0;
            self.soundt0flag = false;
            self.setspkrt2();
        }
    }

    /// C `setsoundmode()`.
    fn setsoundmode(&mut self) {
        self.spkrmode = self.wavetype;
        if !self.soundt0flag && self.sndflag {
            self.soundt0flag = true;
            self.setspkrt2();
        }
    }

    // -- individual effects --------------------------------------------------

    /// C `soundlevdoneoff()`.
    fn soundlevdoneoff(&mut self) {
        self.soundlevdoneflag = false;
        self.soundpausedflag = false;
    }

    /// C `soundlevdoneupdate()`.
    fn soundlevdoneupdate(&mut self) {
        if self.sndflag {
            if self.nljpointer < NEWLEVJINGLE.len() as i32 {
                self.t2val = NEWLEVJINGLE[self.nljpointer as usize] as u16;
            }
            self.t0val = self.t2val.wrapping_add(35);
            self.musvol = 50;
            self.setsoundmode();
            self.sett0();
            let t2val = self.t2val;
            self.sett2val(t2val);
            if self.nljnoteduration > 0 {
                self.nljnoteduration -= 1;
            } else {
                self.nljnoteduration = 20;
                self.nljpointer += 1;
                if self.nljpointer > 10 {
                    self.soundlevdoneoff();
                }
            }
        } else {
            // The C blocking loop would exit and call soundlevdoneoff().
            self.soundlevdoneoff();
        }
    }

    /// C `soundfalloff()`.
    fn soundfalloff(&mut self) {
        self.soundfallflag = false;
        self.soundfalln = 0;
    }

    /// C `soundfallupdate()`.
    fn soundfallupdate(&mut self) {
        if self.soundfallflag {
            if self.soundfalln < 1 {
                self.soundfalln += 1;
                if self.soundfallf {
                    self.t2val = self.soundfallvalue as u16;
                }
            } else {
                self.soundfalln = 0;
                if self.soundfallf {
                    self.soundfallvalue += 50;
                    self.soundfallf = false;
                } else {
                    self.soundfallf = true;
                }
            }
        }
    }

    /// C `soundbreakoff()`.
    fn soundbreakoff(&mut self) {
        self.soundbreakflag = false;
    }

    /// C `soundbreakupdate()`.
    fn soundbreakupdate(&mut self) {
        if self.soundbreakflag {
            if self.soundbreakduration != 0 {
                self.soundbreakduration -= 1;
                self.t2val = self.soundbreakvalue as u16;
            } else {
                self.soundbreakflag = false;
            }
        }
    }

    /// C `soundwobbleoff()`.
    fn soundwobbleoff(&mut self) {
        self.soundwobbleflag = false;
        self.soundwobblen = 0;
    }

    /// C `soundwobbleupdate()`.
    fn soundwobbleupdate(&mut self) {
        if self.soundwobbleflag {
            self.soundwobblen += 1;
            if self.soundwobblen > 63 {
                self.soundwobblen = 0;
            }
            match self.soundwobblen {
                0 => self.t2val = 0x7d0,
                16 | 48 => self.t2val = 0x9c4,
                32 => self.t2val = 0xbb8,
                _ => {}
            }
        }
    }

    /// C `soundfireoff()`.
    fn soundfireoff(&mut self, n: usize) {
        self.soundfireflag[n] = false;
        self.soundfiren[n] = 0;
    }

    /// C `soundfireupdate()`.
    fn soundfireupdate(&mut self) {
        let mut f = false;
        for n in 0..FIREBALLS {
            self.sff[n] = false;
            if self.soundfireflag[n] {
                if self.soundfiren[n] == 1 {
                    self.soundfiren[n] = 0;
                    self.soundfirevalue[n] += self.soundfirevalue[n] / 55;
                    self.sff[n] = true;
                    f = true;
                    if self.soundfirevalue[n] > 30000 {
                        self.soundfireoff(n);
                    }
                } else {
                    self.soundfiren[n] += 1;
                }
            }
        }
        if f {
            loop {
                let n = self.soundfirew;
                self.soundfirew += 1;
                if self.soundfirew == FIREBALLS as i32 {
                    self.soundfirew = 0;
                }
                if self.sff[n as usize] {
                    self.t2val = (self.soundfirevalue[n as usize]
                        + self.randnos(self.soundfirevalue[n as usize] >> 3))
                        as u16;
                    break;
                }
            }
        }
    }

    /// C `soundexplodeoff()`.
    fn soundexplodeoff(&mut self, n: usize) {
        self.soundexplodeflag[n] = false;
    }

    /// C `soundexplodeupdate()`.
    fn soundexplodeupdate(&mut self) {
        let mut f = false;
        for n in 0..FIREBALLS {
            self.sef[n] = false;
            if self.soundexplodeflag[n] {
                if self.soundexplodeduration[n] != 0 {
                    self.soundexplodevalue[n] -= self.soundexplodevalue[n] >> 3;
                    self.soundexplodeduration[n] -= 1;
                    self.sef[n] = true;
                    f = true;
                } else {
                    self.soundexplodeflag[n] = false;
                }
            }
        }
        if f {
            loop {
                let n = self.soundexplodew;
                self.soundexplodew += 1;
                if self.soundexplodew == FIREBALLS as i32 {
                    self.soundexplodew = 0;
                }
                if self.sef[n as usize] {
                    self.t2val = self.soundexplodevalue[n as usize] as u16;
                    break;
                }
            }
        }
    }

    /// C `soundbonusoff()`.
    fn soundbonusoff(&mut self) {
        self.soundbonusflag = false;
        self.soundbonusn = 0;
    }

    /// C `soundbonusupdate()`.
    fn soundbonusupdate(&mut self) {
        if self.soundbonusflag {
            self.soundbonusn += 1;
            if self.soundbonusn > 15 {
                self.soundbonusn = 0;
            }
            if self.soundbonusn >= 0 && self.soundbonusn < 6 {
                self.t2val = 0x4ce;
            }
            if self.soundbonusn >= 8 && self.soundbonusn < 14 {
                self.t2val = 0x5e9;
            }
        }
    }

    /// C `soundemoff()`.
    fn soundemoff(&mut self) {
        self.soundemflag = false;
    }

    /// C `soundemupdate()`.
    fn soundemupdate(&mut self) {
        if self.soundemflag {
            self.t2val = 1000;
            self.soundemoff();
        }
    }

    /// C `soundemeraldoff()`.
    fn soundemeraldoff(&mut self) {
        self.soundemeraldflag = false;
    }

    /// C `soundemeraldupdate()`.
    fn soundemeraldupdate(&mut self) {
        if self.soundemeraldflag {
            if self.soundemeraldduration != 0 {
                if self.soundemeraldn == 0 || self.soundemeraldn == 1 {
                    self.t2val = self.emerfreq as u16;
                }
                self.soundemeraldn += 1;
                if self.soundemeraldn > 7 {
                    self.soundemeraldn = 0;
                    self.soundemeraldduration -= 1;
                }
            } else {
                self.soundemeraldoff();
            }
        }
    }

    /// C `soundgoldoff()`.
    fn soundgoldoff(&mut self) {
        self.soundgoldflag = false;
    }

    /// C `soundgoldupdate()`.
    fn soundgoldupdate(&mut self) {
        if self.soundgoldflag {
            if self.soundgoldduration != 0 {
                self.soundgoldduration -= 1;
            } else {
                self.soundgoldflag = false;
            }
            if self.soundgoldf {
                self.soundgoldf = false;
                self.t2val = self.soundgoldvalue1 as u16;
            } else {
                self.soundgoldf = true;
                self.t2val = self.soundgoldvalue2 as u16;
            }
            self.soundgoldvalue1 += self.soundgoldvalue1 >> 4;
            self.soundgoldvalue2 -= self.soundgoldvalue2 >> 4;
        }
    }

    /// C `soundeatmoff()`.
    fn soundeatmoff(&mut self) {
        self.soundeatmflag = false;
    }

    /// C `soundeatmupdate()`.
    fn soundeatmupdate(&mut self) {
        if self.soundeatmflag {
            if self.soundeatmn != 0 {
                if self.soundeatmduration != 0 {
                    if (self.soundeatmduration % 4) == 1 {
                        self.t2val = self.soundeatmvalue as u16;
                    }
                    if (self.soundeatmduration % 4) == 3 {
                        self.t2val = (self.soundeatmvalue - (self.soundeatmvalue >> 4)) as u16;
                    }
                    self.soundeatmduration -= 1;
                    self.soundeatmvalue -= self.soundeatmvalue >> 4;
                } else {
                    self.soundeatmduration = 20;
                    self.soundeatmn -= 1;
                    self.soundeatmvalue = 2000;
                }
            } else {
                self.soundeatmflag = false;
            }
        }
    }

    /// C `soundddieoff()`.
    fn soundddieoff(&mut self) {
        self.soundddieflag = false;
    }

    /// C `soundddieupdate()`.
    fn soundddieupdate(&mut self) {
        if self.soundddieflag {
            self.soundddien += 1;
            if self.soundddien == 1 {
                self.musicoff();
            }
            if self.soundddien >= 1 && self.soundddien <= 10 {
                self.soundddievalue = 20000 - self.soundddien * 1000;
            }
            if self.soundddien > 10 {
                self.soundddievalue += 500;
            }
            if self.soundddievalue > 30000 {
                self.soundddieoff();
            }
            self.t2val = self.soundddievalue as u16;
        }
    }

    /// C `sound1upoff()`.
    fn sound1upoff(&mut self) {
        self.sound1upflag = false;
    }

    /// C `sound1upupdate()`.
    fn sound1upupdate(&mut self) {
        if self.sound1upflag {
            if (self.sound1upduration / 3) % 2 != 0 {
                self.t2val = ((self.sound1upduration << 2) + 600) as u16;
            }
            self.sound1upduration -= 1;
            if self.sound1upduration < 1 {
                self.sound1upflag = false;
            }
        }
    }

    /// C `musicoff()`.
    fn musicoff(&mut self) {
        self.musicplaying = false;
        self.musicp = 0;
    }

    /// C `musicupdate()`.
    fn musicupdate(&mut self) {
        if !self.musicplaying {
            return;
        }
        if self.noteduration != 0 {
            self.noteduration -= 1;
        } else {
            self.musicstage = 0;
            self.musicn = 0;
            match self.tuneno {
                0 => {
                    self.noteduration = BONUSJINGLE[(self.musicp + 1) as usize] * 3;
                    self.musicnotewidth = self.noteduration - 3;
                    self.notevalue = BONUSJINGLE[self.musicp as usize];
                    self.musicp += 2;
                    if BONUSJINGLE[self.musicp as usize] == TUNE_END {
                        self.musicp = 0;
                    }
                }
                1 => {
                    self.noteduration = BACKGJINGLE[(self.musicp + 1) as usize] * 6;
                    self.musicnotewidth = 12;
                    self.notevalue = BACKGJINGLE[self.musicp as usize];
                    self.musicp += 2;
                    if BACKGJINGLE[self.musicp as usize] == TUNE_END {
                        self.musicp = 0;
                    }
                }
                2 => {
                    self.noteduration = DIRGE[(self.musicp + 1) as usize] * 10;
                    self.musicnotewidth = self.noteduration - 10;
                    self.notevalue = DIRGE[self.musicp as usize];
                    self.musicp += 2;
                    if DIRGE[self.musicp as usize] == TUNE_END {
                        self.musicp = 0;
                    }
                }
                _ => {}
            }
        }
        self.musicn += 1;
        self.wavetype = 1;
        self.t0val = self.notevalue as u16;
        if self.musicn >= self.musicnotewidth {
            self.musicstage = 2;
        }
        match self.musicstage {
            0 => {
                if self.musvol + self.musicattackrate >= self.musicmaxvol {
                    self.musicstage = 1;
                    self.musvol = self.musicmaxvol;
                } else {
                    self.musvol += self.musicattackrate;
                }
            }
            1 => {
                if self.musvol - self.musicdecayrate <= self.musicsustainlevel {
                    self.musvol = self.musicsustainlevel;
                } else {
                    self.musvol -= self.musicdecayrate;
                }
            }
            2 => {
                if self.musvol - self.musicreleaserate <= 1 {
                    self.musvol = 1;
                } else {
                    self.musvol -= self.musicreleaserate;
                }
            }
            _ => {}
        }
        if self.musvol == 1 {
            self.t0val = 0x7d00;
        }
    }

    /// C `randnos()` — the sound subsystem's private PRNG.
    fn randnos(&mut self, n: i32) -> i32 {
        // Sint5 is a 32-bit signed long in the C build, so the multiply wraps.
        self.randvs = self.randvs.wrapping_mul(0x015a_4e35).wrapping_add(1);
        let modulus = n.max(1) as u32;
        (((self.randvs as u32) & 0x7fff_ffff) % modulus) as i32
    }

    // -- PC-speaker emulator -------------------------------------------------

    /// Index the sample lookup table, tolerating the uninitialised tail that
    /// C would read for out-of-range (but never expected) indices.
    #[inline]
    fn lut_at(&self, index: i32) -> u8 {
        self.lut[index.clamp(0, 256) as usize]
    }

    /// C `getsample1()` — the exact workhorse of the PC-speaker emulator.
    fn getsample1(&mut self) -> u8 {
        let mut f = false;
        let mut complicate: u16 = 0;
        let mut spkrt2: u16 = 0;
        let mut noi8: u16 = 0;
        let mut not2: u16 = 0;

        let rate = self.rate;
        let rate_u = rate as u16;

        // Timer 2 square wave / pulse transition.
        if subcarry(&mut self.t2v, rate_u) {
            not2 = self.t2v.wrapping_add(rate_u); // time before the change
            if self.t2f {
                spkrt2 = 0u16.wrapping_sub(self.t2v); // MIN_SAMPs at beginning
                self.t2rate = self.t2new;
                if self.t2rate == (rate_u >> 1) {
                    self.t2v = self.t2rate;
                }
            } else {
                // MIN_SAMPs at end
                spkrt2 = self.t2v.wrapping_add(rate_u);
            }
            self.t2v = self.t2v.wrapping_add(self.t2rate);
            if self.t2rate == (rate_u >> 1) {
                self.t2v = self.t2rate;
            } else {
                self.t2f = !self.t2f;
            }
            complicate |= 1;
        }

        // Timer 0 (int8) interrupt, effectively mode 2.
        if subcarry(&mut self.t0v, rate_u) {
            self.i8flag = true;
            noi8 = self.t0v.wrapping_add(rate_u); // time before the interrupt
            self.t0v = self.t0v.wrapping_add(self.t0rate);
            complicate |= 2;
        }

        let t2sw0 = self.t2sw;

        if self.i8flag && self.i8pulse <= 0 {
            f = true;
            if self.spkrmode != 0 {
                if self.spkrmode != 1 {
                    self.t2sw = !self.t2sw;
                } else {
                    let pw = self.pulsewidth.clamp(0, 50) as usize;
                    self.i8pulse = self.pwlut[pw] as i32;
                    self.t2sw = true;
                    f = false;
                }
            }
        }

        if self.i8pulse > 0 {
            complicate |= 4;
            self.i8pulse -= self.rate;
            if self.i8pulse <= 0 {
                complicate |= 8;
                self.t2sw = false;
                self.i8flag = true;
                f = true;
            }
        }

        if f {
            let add = self.timerrate as u16;
            if addcarry(&mut self.timercount, add) {
                self.soundint(); // update music and sound effects (~72.8 Hz)
                self.timercount = self.timercount.wrapping_sub(0x4000);
            }
            self.i8flag = false;
        }

        if (complicate & 1) == 0 && self.t2f {
            return MIN_SAMP;
        }

        // 12 unique cases, deliberately written without fallthrough.  In the
        // C original no more than about six of these lines execute per call.
        match complicate {
            // Int8 happened.
            2 => {
                if self.t2sw != t2sw0 {
                    if self.t2sw {
                        return self.lut_at(rate - noi8 as i32);
                    }
                    return self.lut_at(noi8 as i32);
                }
                // Fall through to case 0.
                if !self.t2sw {
                    return MIN_SAMP;
                }
                MAX_SAMP
            }
            // Nothing happened.
            0 => {
                if !self.t2sw {
                    return MIN_SAMP;
                }
                MAX_SAMP
            }
            // Int8 is pulsing => t2sw.
            4 => MAX_SAMP,
            // The t2 wave changed.
            1 => {
                if !self.t2sw {
                    return MIN_SAMP;
                }
                // Fall through to case 5.
                self.lut_at(spkrt2 as i32)
            }
            // The t2 wave changed and int8 is pulsing => t2sw.
            5 => self.lut_at(spkrt2 as i32),
            // Int8 happened and the t2 wave changed.
            3 => {
                if !t2sw0 && !self.t2sw {
                    return MIN_SAMP; // both parts are off
                }
                if t2sw0 && self.t2sw {
                    return self.lut_at(spkrt2 as i32); // both parts are on
                }
                if not2 < noi8 {
                    // t2 happened first.
                    if t2sw0 {
                        // "on" part is before i8.
                        if self.t2f {
                            return self.lut_at(spkrt2 as i32); // MIN at end
                        }
                        return self.lut_at(spkrt2 as i32 - (rate - noi8 as i32));
                    }
                    // "on" part is after i8 => constant.
                    if self.t2f {
                        return MIN_SAMP; // MIN at end
                    }
                    return self.lut_at(rate - noi8 as i32); // MIN at start
                } else {
                    // i8 happened first.
                    if t2sw0 {
                        // "on" part is before i8 => constant.
                        if self.t2f {
                            return MIN_SAMP; // MIN at beginning
                        }
                        return self.lut_at(noi8 as i32); // MIN at end
                    }
                    // "on" part is after i8.
                    if self.t2f {
                        return self.lut_at(spkrt2 as i32); // MIN at start
                    }
                    return self.lut_at(spkrt2 as i32 - noi8 as i32); // MIN at end
                }
            }
            // The int8 pulse started.
            6 => {
                if t2sw0 {
                    return MAX_SAMP;
                }
                self.lut_at(rate - noi8 as i32)
            }
            // The int8 pulse started and the t2 wave changed.
            7 => {
                if t2sw0 {
                    return self.lut_at(spkrt2 as i32);
                }
                if not2 < noi8 {
                    // t2 happened first.
                    if self.t2f {
                        return MIN_SAMP; // MIN at end
                    }
                    return self.lut_at(rate - noi8 as i32); // MIN at start
                } else {
                    // i8 happened first.
                    if self.t2f {
                        return self.lut_at(spkrt2 as i32); // MIN at start
                    }
                    return self.lut_at(spkrt2 as i32 - noi8 as i32); // MIN at end
                }
            }
            // The int8 pulse stopped.
            12 => {
                if self.t2sw {
                    return MAX_SAMP;
                }
                self.lut_at(self.i8pulse + rate as i32)
            }
            // The int8 pulse stopped and the t2 wave changed.
            13 => {
                if self.t2sw {
                    return self.lut_at(spkrt2 as i32);
                }
                if (not2 as i32) < self.i8pulse + rate {
                    // t2 happened first.
                    if self.t2f {
                        return self.lut_at(spkrt2 as i32 + self.i8pulse);
                    }
                    return self.lut_at(spkrt2 as i32); // MIN at end
                } else {
                    // i8pulse ended first.
                    if self.t2f {
                        return MIN_SAMP; // MIN at start
                    }
                    return self.lut_at(self.i8pulse + rate);
                }
            }
            // The int8 pulse started and stopped in the same sample.
            14 => {
                if t2sw0 {
                    if self.t2sw {
                        return MAX_SAMP;
                    }
                    return self.lut_at(noi8 as i32 + self.i8pulse + rate);
                } else {
                    if self.t2sw {
                        return self.lut_at(rate - noi8 as i32);
                    }
                    return self.lut_at(self.i8pulse + rate);
                }
            }
            // Everything happened at once.
            15 => {
                if not2 < noi8 {
                    // First subcase: t2 happens before the pulse.
                    if self.t2f {
                        // MIN at beginning.
                        if t2sw0 {
                            if self.t2sw {
                                return self.lut_at(spkrt2 as i32);
                            }
                            return self.lut_at(spkrt2 as i32 + noi8 as i32 + self.i8pulse);
                        } else {
                            if self.t2sw {
                                return self.lut_at(rate - noi8 as i32);
                            }
                            return self.lut_at(self.i8pulse + rate);
                        }
                    } else {
                        // MIN at end.
                        if t2sw0 {
                            // No need to test t2sw.
                            return self.lut_at(spkrt2 as i32);
                        }
                        return MIN_SAMP;
                    }
                } else if (not2 as i32) < rate + noi8 as i32 + self.i8pulse {
                    // Subcase 2: t2 happens during the pulse.
                    if self.t2f {
                        // MIN at beginning.
                        if self.t2sw {
                            // No need to test t2sw0.
                            return self.lut_at(spkrt2 as i32);
                        }
                        return self.lut_at(spkrt2 as i32 + noi8 as i32 + self.i8pulse);
                    } else {
                        // MIN at end.
                        if t2sw0 {
                            // No need to test t2sw.
                            return self.lut_at(spkrt2 as i32);
                        }
                        return self.lut_at(spkrt2 as i32 - noi8 as i32);
                    }
                } else {
                    // Third subcase: t2 happens after the pulse.
                    if self.t2f {
                        // MIN at beginning.
                        if self.t2sw {
                            // No need to test t2sw0.
                            return self.lut_at(spkrt2 as i32);
                        }
                        return MIN_SAMP;
                    } else {
                        // MIN at end.
                        if t2sw0 {
                            if self.t2sw {
                                return self.lut_at(spkrt2 as i32);
                            }
                            return self.lut_at(noi8 as i32 + self.i8pulse + rate);
                        } else {
                            if self.t2sw {
                                return self.lut_at(spkrt2 as i32 - noi8 as i32);
                            }
                            return self.lut_at(self.i8pulse + rate);
                        }
                    }
                }
            }
            // This should never happen.
            _ => MIN_SAMP,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// (1) With sound and music disabled and no effects, output is silent.
    #[test]
    fn silence_when_disabled() {
        let mut engine = SoundEngine::new(44100);
        engine.set_music_enabled(false);
        engine.set_sound_enabled(false);
        engine.sound_stop();

        let mut buffer = [1.0f32; 8192];
        engine.render(&mut buffer);
        assert!(buffer.iter().all(|sample| *sample == 0.0));
    }

    /// (2) The background tune produces non-silent, varying samples.
    #[test]
    fn music_produces_varying_samples() {
        let mut engine = SoundEngine::new(44100);
        engine.music(1);

        let mut buffer = [0.0f32; 8192];
        engine.render(&mut buffer);

        let min = buffer.iter().copied().fold(f32::INFINITY, f32::min);
        let max = buffer.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(max > 0.0, "expected non-silent output, got max {max}");
        assert!(max > min, "expected a varying waveform");
    }

    /// (2b) The extra-life jingle is audible too.
    #[test]
    fn one_up_produces_sound() {
        let mut engine = SoundEngine::new(44100);
        engine.sound_stop();
        engine.one_up();

        let mut buffer = [0.0f32; 4096];
        engine.render(&mut buffer);
        assert!(buffer.iter().any(|sample| *sample != 0.0));
    }

    /// (3) The level-done jingle terminates on its own.
    #[test]
    fn level_done_finishes() {
        let mut engine = SoundEngine::new(44100);
        engine.level_done_start();
        assert!(engine.level_done_active());

        let mut buffer = [0.0f32; 4096];
        let mut rendered = 0usize;
        // 11 notes x ~21 updates at 72.8 Hz is roughly 3.2 s; allow plenty.
        while engine.level_done_active() && rendered < 44100 * 12 {
            engine.render(&mut buffer);
            rendered += buffer.len();
        }
        assert!(!engine.level_done_active(), "jingle did not finish");

        // It stays finished and rendering remains safe.
        engine.render(&mut buffer);
        assert!(!engine.level_done_active());
    }

    /// (4) Starting an emerald effect produces non-silence.
    #[test]
    fn emerald_produces_sound() {
        let mut engine = SoundEngine::new(44100);
        engine.music_off();
        engine.sound_stop();
        engine.emerald(0);

        let mut buffer = [0.0f32; 4096];
        engine.render(&mut buffer);
        assert!(buffer.iter().any(|sample| *sample != 0.0));
    }
}
