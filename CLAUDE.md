# CLAUDE.md

Guidance for working in this repository.

## Project

Cross-platform Rust remake of **Digger** (1983, Windmill Software), based on the
**Digger Remastered VGA** source in `ref/remastered`. Goals:

- Faithful simulation and feel (same speeds, timings, quirks, scoring).
- Original VGA art (`vga.spr`, `vgatext.spr`, `vtitle.bmp`) and original
  PC-speaker sound emulation.
- Pure Rust for application code, windowing and audio: `winit` + `softbuffer`
  for the window/framebuffer, `cpal` for output. No SDL/SFML/raylib.
- Modern displays: fixed 640x400 internal framebuffer, nearest-neighbour
  integer scaling, letterboxed.

**Self-containment rule:** everything the game needs at build/run time must
live in this repository outside `ref/`. `ref/` is a development-only archival
tree: it is git-ignored, excluded from `cargo package`, and must never be
required by the build or shipped in a release. All runtime assets are embedded
from `assets/` via `include_bytes!`.

License: **GPL-2.0-or-later** (inherited from Digger Remastered). Asset provenance
and the Windmill Software notice are in `assets/ATTRIBUTION.md`; the upstream
GPL text is `LICENSE` and `assets/COPYING.digger-remastered`. **Do not edit
`ref/`**; it is archival reference material.

## Commands

```sh
cargo run --release        # play
cargo run                  # play with debug assertions
cargo test                 # unit + integration + self-containment tests
cargo test --lib           # unit tests only (fast)
cargo check                # fast type check
cargo run --example headless -- --frames 300 --dir right --out /tmp/game.ppm
scripts/check_release.sh   # verify the package excludes ref/ and tests pass
scripts/build_binaries.sh  # build host-possible release binaries into dist/
cargo fmt
cargo clippy --all-targets
```

Development-only environment variables (used by the windowed app and the
headless example):

- `DIGGER_AUTOSTART=1` — skip the title screen.
- `DIGGER_START_LEVEL=N` — start on level N.
- `DIGGER_AUTO_DIR=right|up|left|down` — hold a direction for player 1.
- `DIGGER_AUTO_FIRE=1` — hold the fire button.
- `DIGGER_DUMP=/path/frame.ppm` + `DIGGER_DUMP_AT=N` — write frame N as PPM.
- `DIGGER_DATA_DIR=/path` — where `digger.sco` is read/written.

None of these are required at runtime; they exist for visual regression work
and CI. The headless runner is `examples/headless.rs`.

High scores persist to `digger.sco` in the working directory (override the
directory with `DIGGER_DATA_DIR`).

## Current status (update this section when milestones land)

- [x] Project scaffold, docs, build, self-containment checks
- [x] Asset decoding (`assets.rs`) + palette (`palette.rs`)
- [x] Software renderer / field blobs / text / title bitmap (`screen.rs`)
- [x] Sprite records + collisions (`sprites.rs`)
- [x] PC-speaker emulator + sound state machine (`sound.rs`, C-sample-identical)
- [x] Game flow, player, fire, monsters, bags, bonus, scoring (`game/`)
- [x] Title screen animation, high scores, alternating and simultaneous 2P
- [x] Headless runner (`examples/headless.rs`) + frame dumps
- [x] Player-1 gamepad support (`gilrs`)
- [x] macOS universal + Windows x86_64 binaries on the Mac host
- [ ] Native Linux binary (CI workflow is set up; needs a Linux runner)
- [ ] Manual GUI/audio playtest on each platform
- [ ] Gauntlet mode, level-file support, input remapping
- [ ] Packaging/release CI

## Public mirror

The public repository (the `digger` git submodule, `github.com/kiltum/digger`)
is a generated mirror of the self-contained sources. It intentionally excludes
`ref/`, build outputs and local score files. The private development tree uses
`scripts/sync_public.sh` (not itself part of the mirror) to update it; run
`scripts/sync_public.sh --check` to verify the mirror is current.

## Source of truth

- `ref/remastered/` — **primary** reference (C, VGA, mature logic).
- `ref/windows/` — same engine with the Windows/DirectDraw backend; useful for
  generated sprite tables (`vgagrafx.c`, `cgagrafx.c`).
- `ref/original/` — 1983-era assembly/C. Useful for understanding intent and
  the original asset pipeline, but **not** the porting source.
- `ref/remastered/PORTING_NOTES` equivalent: see `docs/PORTING_NOTES.md`.
- `docs/ARCHITECTURE.md` — module map and data flow.
- `docs/PORTING_NOTES.md` — detailed C-to-Rust porting reference.
- `examples/headless.rs` — runs the simulation without a window (CI/debug).
- `scripts/check_release.sh` — packaging self-containment check.

## Architecture at a glance

```
UI thread (winit)                 Game thread (~15 Hz)              Audio callback
-----------------                 --------------------              --------------
keyboard -> SharedInput   ---->   Sys::newframe()                   SoundEngine
RedrawRequested                   input.poll / direction latches    | render()
  |                               simulation (game::*)
  |<---- Shared frame <---------- screen.compose()
softbuffer present                "UserEvent::Frame"
```

- `main.rs` — winit `ApplicationHandler`, softbuffer presentation, key events.
- `platform.rs` — `Shared` (latest RGBA frame + ui events), `SharedInput`.
- `sys.rs` — game-thread services: clock, input, screen, sprite system, sound
  command wrappers.
- `screen.rs` — indexed 640x400 renderer. Two layers:
  - `bg`: dirt pattern, dug tunnels, emeralds, HUD/title text (mutated by game logic)
  - `fb`: per-frame `bg` copy + enabled sprite records, converted with palette
- `sprites.rs` — port of `sprite.c` state + `bcollides` collision lists.
- `assets.rs` — decode `vga.spr` (107 records) / `vgatext.spr` (40 glyphs) /
  `vtitle.bmp` (BI_RLE4) and map game sprite numbers 0..119 to raw records.
- `palette.rs` — four original 16-colour palettes and `vgawrite` colour remap.
- `input.rs` — held keys + one-shot edges; the original static/dynamic direction
  latches per player; `PadState` for player-1 gamepad input.
- `gamepad.rs` — `gilrs` polling thread; maps the first connected controller
  (stick/D-pad/fire/Start) onto player 1 only.
- `sound.rs` — exact port of `newsnd.c::getsample1` + `sound.c` state machine,
  advanced at 72.8 Hz.
- `audio.rs` — cpal output; silent fallback thread if no device.
- `game/` — `mod.rs` (flow, levels, HUD, score helpers), `digger.rs`,
  `monster.rs`, `bags.rs`, `scores.rs`.
- `levels.rs` — the eight original layouts. `constants.rs` — shared constants.

## Fidelity rules

1. **Port, don't redesign.** Keep function names and structure aligned with the
   C modules where practical (`updatedigger`, `monai`, `pushbag`, ...).
2. **Preserve quirks.** Examples: extra life triggers on `score >= nextbs+n`
   (the `+n` is an original bug we keep); 250/25/500/1000/200×msc scores;
   `level > 8` plan `(l&3)+5`; bag cleanup rules; monster penalty kludges.
3. **The original's `drawspr` side effects matter.** Logic copies `first[]` and
   `coll[]` immediately after a draw call. Our `SpriteSystem::drawspr` must
   recompute `bcollides` every time, and call sites must copy the arrays after.
4. **`draw*` helpers only update sprite records.** They do not paint pixels;
   `Screen::compose` paints all enabled records each frame. `erasespr` only
   disables a record.
5. **Coordinates are logical 320x200.** Never multiply by 2 in game code; the
   renderer does that. Sprite art is stored at 2x (32 px ≈ 16 logical px).
6. **Field bits:** low 5 bits = five 4px columns, bits 6..11 = six 3px rows,
   bit 0x2000 = "cell is solid / not a path". `makefield` uses `0xd03f` and
   `0xdfe0`; `eatfield` uses `bitmasks[]`; `fieldclear` uses the direction
   tests in `monster.c`.
7. **Timing:** `ftime=80000` PIT ticks (~14.9 fps); `PIT_HZ=1193181`.
   `newframe()` waits one tick, polls input, composes and publishes.
8. **Sound timing is independent:** the audio callback advances the speaker
   emulator; `soundint` runs every 0x4000 PIT ticks (~72.8 Hz). Do not drive
   music from the game tick.
9. **Two-player bookkeeping:** `diggers` = simultaneous diggers (1 or 2),
   `nplayers` = alternating players (1 or 2). `curplayer` offsets global digger
   slots. Sprite indices are relative to `curplayer` for draw/erase.

## Asset/sprite facts (hard-won)

- `vga.spr` = 107 records × 1024 bytes, in `mkg.c`'s `gnames[]` order.
- Normal sprites decode as: pixel `(x,y)` = byte `rec[y*32+x]`, where output
  width may exceed 32 (e.g. blobs are 48 px wide); `0xff` = transparent.
- Raw 80..85 are **masks** (rightblob, leftblob, topblob, bottomblob,
  furryblob, squareblob): 0x0f = transparent, 0 = opaque **black**.
- Raw 87..94 are background tiles with a special 40x8 layout: first 32 px from
  `rec[y*32 + 0..32]`, remaining 8 px from `rec[y*32 + 512 + 0..8]`.
- Text is 40 glyphs, charset `ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.:_ `,
  each glyph the top-left 24x24 of a raw record. Lowercase folds to uppercase.
- Game sprite numbers: 0 bonus, 1..50 bags, 51..56 monsters, 57..58 fireballs,
  59..60 diggers; 62..68 bag/gold frames, 69..80 monsters, 82..93 fire/exp,
  94..101 backgrounds, 102..107 blobs, 108 emerald, 109 erase-emerald,
  110..113 lives, 114..119 P2 fire/explosion. Exact table: `src/assets.rs`.
- Palettes are 16 colours; `palette index`/`intensity` select one of four.
  Text colour remap happens per glyph (`palette::text_color`).

## Threading/borrow notes

- `SharedInput` is behind a `Mutex`; the UI thread pushes events/held keys, the
  game thread drains once per tick.
- `SoundEngine` is behind `Mutex<SoundEngine>` shared by game thread (commands)
  and audio callback (render). Keep critical sections short.
- The game thread owns `Sys`/`Game`; only the latest RGBA frame and input cross
  threads. The UI thread never touches game state.
- Prefer disjoint-field borrows (`self.sys.screen...` / `self.sys.sprites...`)
  or small helper methods over splitting structs.

## When adding features

- Put new gameplay in the matching `game/` module as `impl Game` methods and
  keep the C function name in a comment.
- Add pure helpers/tests in the module that owns the data.
- Anything timing-sensitive must use `Sys::newframe` or the PIT constants.
- Keep `cargo test --lib` green and warning-light.
- Update `docs/PORTING_NOTES.md`/`ROADMAP.md` and this status list when a
  milestone lands.
