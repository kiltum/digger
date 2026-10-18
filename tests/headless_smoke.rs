//! Smoke test: run the real simulation headlessly for a couple of seconds of
//! game time. This catches panics in field/monster/bag/digger code and checks
//! that a frame is produced.

use std::sync::{Arc, Mutex};

use digger::constants::{FB_H, FB_W};
use digger::game::Game;
use digger::platform::Shared;
use digger::sound::SoundEngine;
use digger::sys::Sys;

#[test]
fn runs_the_first_level_headlessly() {
    let shared = Shared::headless();
    let sound = Arc::new(Mutex::new(SoundEngine::new(48_000)));
    let mut sys = Sys::new_headless(shared, Some(sound), Some(180)).expect("assets load");
    sys.set_debug_dir(0); // walk right
    let mut game = Game::new(sys);
    game.startlev = 1;
    game.run_headless();

    assert_eq!(game.frame().len(), FB_W * FB_H);
    assert!(game.frame().iter().any(|&p| p != 0), "frame is blank");
    let summary = game.debug_summary();
    assert!(summary.contains("level=1"), "unexpected summary: {summary}");
    // The digger walks for ~12 seconds; it should have picked something up or
    // at least still be in a valid state.
    assert!(game.sys.clock.frame > 0);
}

#[test]
fn alternating_two_player_runs_without_panicking() {
    // Regression: `monai` used to underflow when the chased slot belonged to
    // the inactive player in alternating mode.
    for seed in 1..=8 {
        let shared = Shared::headless();
        let sound = Arc::new(Mutex::new(SoundEngine::new(48_000)));
        let mut sys = Sys::new_headless(shared, Some(sound), Some(1200)).expect("assets load");
        sys.set_debug_dir(6); // hold down
        let mut game = Game::new(sys);
        game.nplayers = 2;
        game.startlev = 1;
        game.rng_seed = Some(seed);
        game.run_headless();
        assert_eq!(game.frame().len(), FB_W * FB_H);
    }
}
