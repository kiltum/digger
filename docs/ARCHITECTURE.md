# Architecture

## Goals

1. **Fidelity.** The simulation is a direct port of Digger Remastered's VGA
   build (`ref/remastered`), including its timing, scoring and quirks.
2. **Original presentation.** 640x400 indexed framebuffer, original sprites,
   original 16-colour palettes, original PC-speaker waveform emulation.
3. **Portability.** Only Rust application code. Windowing (`winit`), software
   presentation (`softbuffer`) and audio output (`cpal`) are the only
   dependencies; all game/asset/sound logic is in this crate.

## Threads

```
┌─────────────────────────────┐
│ UI thread (winit)           │
│  - window / input events    │
│  - softbuffer present       │
│  - integer scaling          │
└─────────────┬───────────────┘
              │ SharedInput (held keys + event queue)
              │ Shared frame (0x00RRGGBB, 640x400) + UserEvent::Frame
              ▼
┌─────────────────────────────┐        commands        ┌──────────────────────┐
│ Game thread (~14.9 Hz)      │ ─────────────────────► │ Audio callback       │
│  Sys::newframe              │                        │  SoundEngine::render │
│  input poll / latches       │ ◄───────────────────── │  (~72.8 Hz updates)  │
│  game::* simulation         │    (shared state)      └──────────────────────┘
│  Screen::compose            │
└─────────────────────────────┘
```

The game thread is synchronous and follows the original's blocking control
flow: `newframe()` sleeps until the next PIT-derived tick, then polls input,
composes a frame and publishes it. This made a near line-by-line port of the
C loops possible.

## Rendering

The renderer keeps palette indices until the final blit.

- `Screen.bg` — **destructible layer**. Contains:
  - the dirt texture (background tiles 94..101),
  - dug tunnels (black blobs 102..107, carved by `drawfield`/`draw*blob`),
  - emeralds (sprite 108 / erase shape 109),
  - HUD/title text and the title bitmap.
- `Screen.fb` — per frame: `bg.copy_from_slice`, then every enabled sprite
  record (`SpriteSystem`) is blitted in index order (bonus, bags, monsters,
  fireballs, diggers).
- Presentation converts `fb` through the active palette (or the title bitmap's
  palette while the title screen is shown) to `0x00RRGGBB` and publishes it.

The original save/restore buffers are therefore unnecessary: it is cheaper and
simpler to rebuild the composite each frame at ~15 fps.

`Screen::getpix` samples the dirt layer like `vgagetpix` (8x2 physical pixels,
masked with `0xee`) for fireball wall detection.

### Scaling

The UI thread fits the 640x400 frame into the window preserving aspect ratio.
When scale ≥ 1 it uses the largest integer factor that fits; otherwise it falls
back to nearest-neighbour non-integer scaling. Bars are filled black.

## Simulation layout

| Rust | C reference | Responsibility |
|---|---|---|
| `game/mod.rs` | `main.c` + bits of `drawing.c` | game flow, level setup, HUD, scoring helpers |
| `game/digger.rs` | `digger.c` | player movement, fire, death, bonus, emeralds |
| `game/monster.rs` | `monster.c` | Nobbin/Hobbin creation and AI |
| `game/bags.rs` | `bags.c` | bag wobble/fall/push, gold |
| `game/scores.rs` | `scores.c` | points, extra lives, high scores |
| `sprites.rs` | `sprite.c` | entity records + `bcollides` |
| `screen.rs` | `drawing.c` (+ `win_vid.c`) | field blobs, text, palette blit |
| `assets.rs` | `mkg.c` + asset files | sprite/text/title decoding |
| `palette.rs` | `win_vid.c` | palettes + `vgawrite` colour remap |
| `input.rs` | `input.c` | key mapping and direction latches |
| `sound.rs` | `newsnd.c` + `sound.c` | speaker emulation + music/effects |
| `sys.rs` | platform glue | clock, input, screen and sound API |

### Sprite records

`SpriteSystem` keeps `sprenf`, `sprx/y/w/h`, `sprbw/bh`, `sprn*`, plus the
`first[TYPES]` / `coll[SPRITES]` lists filled by `bcollides`. Draw helpers
(`drawdigger`, `drawmon`, `drawgold`, `drawfire`) only call `initspr` +
`drawspr`; `drawspr` recomputes collisions. Logic must copy `first`/`coll`
immediately after a draw, exactly as the C does.

Entity sprite numbers:
`0` bonus, `1..50` bags, `51..56` monsters, `57..58` fireballs, `59..60` diggers.

### Field model

15x10 cells, each 20x18 logical pixels. A cell is a 16-bit mask:

```
bit 15..12 : flags (0x2000 = solid / not a path)
bits 11..6 : six horizontal strips of 3 logical pixels
bits  5..0 : five vertical strips of 4 logical pixels
```

Level characters: ` ` solid dirt, `H` horizontal tunnel, `V` vertical tunnel,
`C` emerald, `B` bag, `S` start. `makefield()` masks `0xffff` with `0xd03f`
(H/V) and/or `0xdfe0` (H/S), then `drawfield()` carves the tunnel borders with
blob sprites. `eatfield()` clears individual strip bits; `fieldclear()` tells
monsters where they may move.

## Timing

- `PIT_HZ = 1_193_181`.
- Game frame `ftime = 80_000` ticks ⇒ ~14.9 fps (adjustable with `-`/`=`).
- `Clock` converts `ftime` to `Duration` and sleeps to the next tick.
- Gauntlet time (`cgtime`) is kept in PIT ticks and decremented by `ftime` per
  frame.
- The sound emulator advances at `samplerate` and calls `soundint` every
  `0x4000` PIT ticks (~72.8 Hz), matching `newsnd.c`.

## Audio

`SoundEngine` is a direct port of `getsample1()` (8253 PIT + PC speaker PWM)
and the `sound.c` state machine. The game thread sends commands (music, effect
start/stop) through a mutex; the audio callback renders f32 samples. If no
output device exists, a silent thread renders to keep timing alive.

## Adding a feature

1. Find the corresponding C function in the reference.
2. Port it into the matching `game/` module as `impl Game`, preserving names,
   constants and ordering.
3. If it touches rendering, put static art in `Screen.bg` and entities in
   sprite records.
4. If it touches sound, add a command to `SoundEngine` in `sound.rs` and a
   wrapper in `sys.rs`.
5. Add a unit test where the logic is pure (field bits, collision, scoring).
