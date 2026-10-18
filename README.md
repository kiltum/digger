# Digger (Rust remake)

A cross-platform remake of **Digger**, the 1983 PC classic by Windmill
Software, written in Rust. It uses the original VGA artwork and an emulation of
the original PC-speaker sound, driven by a faithful port of the community
*Digger Remastered* source.

> Status: playable single player (title, levels, monsters, bags, scoring,
> sound). See `docs/ROADMAP.md` for the remaining parity work.

## Features

- Original 8 level layouts, monsters (Nobbin/Hobbin), bags, gold, emeralds,
  bonus mode, fireballs, lives, scoring and high scores.
- Original VGA sprites and palettes, pixel-for-pixel.
- Original PC-speaker music and sound effects, emulated from the PIT waveforms.
- One-player, alternating two-player and simultaneous two-player modes.
- Crisp nearest-neighbour scaling, resizable window, letterboxing.

Everything the game needs at build/run time is in `src/` and `assets/` and is
embedded into the binary. The archival `ref/` tree is development-only: it is
git-ignored, excluded from `cargo package`, and never needed to build or run the
game.

## Build & run

Requires a Rust toolchain (1.86+).

```sh
cargo run --release
```

On first run the game writes high scores to `digger.sco` in the working
directory.

## Controls

| Action | Player 1 | Player 2 |
|---|---|---|
| Move | Arrow keys | W / A / S / D |
| Fire | Space, Ctrl | Left Shift |
| Gamepad (P1) | Left stick / D-pad, A/B/triggers fire, Start starts | |
| Pause | P | P |
| Music on/off | F7 | |
| Sound on/off | F9 | |
| Speed down / up | `-` / `=` | |
| Fullscreen | F11 | |
| Quit | Esc | |

On the title screen press **N** to toggle alternating one/two players, **S**
to toggle simultaneous two-player mode, and any other key to start.

## Layout

```
src/            Rust implementation
  assets.rs     vga.spr / vgatext.spr / vtitle.bmp decoding
  palette.rs    original 16-colour palettes
  screen.rs     640x400 indexed renderer (dirt, tunnels, sprites, text)
  sprites.rs    sprite records + collision lists (port of sprite.c)
  sound.rs      PC-speaker emulator + music/effects state machine
  audio.rs      cpal output stream
  input.rs      keyboard state and original direction latches
  sys.rs        game-thread services (clock, screen, sound commands)
  game/         simulation: flow, digger, monsters, bags, scores
  levels.rs     original level data
  constants.rs  shared constants
assets/         original VGA assets + upstream GPL text + attribution
docs/           architecture, porting notes, roadmap
ref/            development-only reference sources (git-ignored, never shipped)
```

## Development

```sh
cargo test                     # unit + integration + self-containment tests
cargo run --example headless -- --frames 300 --dir right --out /tmp/game.ppm
scripts/check_release.sh       # verify ref/ is excluded and everything passes
scripts/build_binaries.sh      # macOS universal + Windows x86_64 from a Mac
```

`scripts/build_binaries.sh` produces what the host can build. On this macOS
host that is `dist/digger-macos-universal` (arm64 + x86_64) and
`dist/digger-windows-x86_64.exe` (mingw-w64). Linux binaries are built by the
native Ubuntu job in `.github/workflows/release.yml` (a macOS host cannot
cross-link ALSA/libudev/X11 without a Linux sysroot). macOS binaries are
unsigned; if one is downloaded and quarantined, run
`xattr -d com.apple.quarantine dist/digger-macos-universal`.

The headless runner drives the real simulation and writes a PPM frame, which is
useful for CI and visual checks. See `CLAUDE.md` for the other development
environment variables.

## Credits and licensing

This is a preservation/remake project based on **Digger Remastered** by Andrew
Jenner, which in turn is based on the original **Digger** by Windmill Software.
The code is distributed under the **GNU General Public License v2.0 or later** (see
`LICENSE`). The graphics and sound data remain the property of their respective
owners; see `assets/ATTRIBUTION.md` for provenance and the licensing notes from
the upstream release.

The project is not affiliated with or endorsed by Windmill Software or the
Digger Remastered authors.
