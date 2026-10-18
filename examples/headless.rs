//! Headless runner for development, CI and visual regression work.
//!
//! Examples:
//!   cargo run --example headless -- --frames 300 --dir right --out /tmp/game.ppm
//!   cargo run --example headless -- --frames 50 --level 3 --out /tmp/l3.ppm

use std::sync::{Arc, Mutex};

use digger::game::Game;
use digger::platform::Shared;
use digger::sound::SoundEngine;
use digger::sys::Sys;

fn arg(name: &str) -> Option<String> {
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        if a == name {
            return it.next();
        }
        if let Some(v) = a.strip_prefix(&format!("{name}=")) {
            return Some(v.to_string());
        }
    }
    None
}

fn main() {
    let frames: u64 = arg("--frames").and_then(|s| s.parse().ok()).unwrap_or(200);
    let out = arg("--out").unwrap_or_else(|| "/tmp/digger-headless.ppm".to_string());

    if let Some(level) = arg("--level") {
        std::env::set_var("DIGGER_START_LEVEL", level);
    }
    if let Some(dir) = arg("--dir") {
        std::env::set_var("DIGGER_AUTO_DIR", dir);
    }
    if let Some(seed) = arg("--seed") {
        std::env::set_var("DIGGER_SEED", seed);
    }
    if arg("--fire").is_some() {
        std::env::set_var("DIGGER_AUTO_FIRE", "1");
    }

    let shared = Shared::headless();
    let sound = Arc::new(Mutex::new(SoundEngine::new(48_000)));
    let sys = Sys::new_headless(shared, Some(sound), Some(frames))
        .unwrap_or_else(|e| panic!("failed to load assets: {e}"));
    let mut game = Game::new(sys);
    if let Some(p) = arg("--players") {
        if let Ok(n) = p.parse::<i32>() {
            game.nplayers = n.clamp(1, 2);
        }
    }
    if arg("--simultaneous").is_some() {
        game.diggers = 2;
        game.nplayers = 1;
    }
    game.run_headless();

    let frame = game.frame();
    let mut ppm = Vec::with_capacity(frame.len() * 3 + 32);
    ppm.extend_from_slice(
        format!(
            "P6\n{} {}\n255\n",
            digger::constants::FB_W,
            digger::constants::FB_H
        )
        .as_bytes(),
    );
    for &px in frame {
        ppm.push((px >> 16) as u8);
        ppm.push((px >> 8) as u8);
        ppm.push(px as u8);
    }
    std::fs::write(&out, ppm).unwrap_or_else(|e| panic!("failed to write {out}: {e}"));

    println!("{}", game.debug_summary());
    println!("wrote {out}");
}
