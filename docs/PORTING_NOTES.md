# Digger porting notes

Engineering reference for the cross-platform Rust remake. This document records
the parts of the reference implementation that are easy to lose: exact units,
bit layouts, ordering, constants, and intentional quirks. Unless stated
otherwise, line references are to `ref/remastered/`.

---

## 1. Reference trees and primary source

| Tree | What it is | How to use it |
|---|---|---|
| `ref/remastered/` | **Digger Remastered** by Andrew Jenner (1998-2004): DOS/ARM/Windows C sources, the VGA/CGA sprite assets, `mkg.c`, `routines.txt`, `digsrc.txt`, GPL-2.0 text (`COPYING`). | **Primary porting source.** The VGA C code and the VGA assets in this tree define the behaviour and the art this port reproduces. |
| `ref/original/` | The original 1983 Windmill Software development tree: `Source/` (C and ASM modules, `DIGGER.H`, `TITLE.ASM`, `SOUND.ASM`, `GRAPHLIB.ASM`, ...), `Work/` (TA.C..TZ.C task-based prototype), `Sprites/`, `C Compiler/`, `Disk Create/` (original `DIGGER.EXE`/`DIGGER.BIN`), and the `CLib` character-glyph data. | Historical reference only. Useful when Remastered's code is ambiguous, but do not port ASM-level artefacts or original memory layouts. |
| `ref/windows/` | A later/alternate Windows build tree (Tim Draper era): menu and config dialogs (`win_cfg.c`, `WinDig.rc`), generated `vgagrafx.c`/`cgagrafx.c`/`alpha.c`, older DirectDraw interface, `supported_video_modes`, `suspend_game`, 19 remappable commands, `MapVirtualKey` input handling. | Reference for Windows-specific UI/input/video features only. It diverges from Remastered in `main.c`, `digger.c`, `scores.c`, `input.c`, `win_vid.c`, `win_snd.c`; do not use it as the gameplay source. |

The VGA build in `ref/remastered/` is the source of truth because:

* It contains the uncompiled VGA assets (`vga.spr`, `vgatext.spr`, `vtitle.bmp`)
  and the generator (`mkg.c`) that documents their layout.
* `routines.txt` documents the terminology, units, field model, and every
  module's responsibilities.
* The Windows-only tree embeds generated graphics and adds UI behaviour that is
  not part of the original game.

The port uses `assets/vga.spr`, `assets/vgatext.spr`, and `assets/vtitle.bmp`,
which are byte-identical to the copies in `ref/remastered/` (see
`assets/ATTRIBUTION.md`).

### Build shape of the C program

```
main.c        program shell, title screen, level flow, level data, INI/CLI
  digger.c    diggers, fireballs, emeralds, death, bonus mode
  monster.c   nobbins/hobbins (AI, spawning, death)
  bags.c      bags, wobbling, falling, gold, pushing
  drawing.c   field bitmask, blob carving, entity drawing, HUD
    sprite.c  sprite records, save/restore screen buffers, collision lists
  scores.c    score, extra lives, high-score table
  input.c     keyboard/joystick state machines
  sound.c     high-level music/effects state machines (72.8 Hz)
    newsnd.c  PC-speaker/8253 emulator + sample generation
  win_vid.c   VGA framebuffer, palettes, glyphs, sprite blitting
  win_snd.c   Windows waveOut/DirectSound output
  title.c     DOS RLE title-screen data
  record.c    DRF input recording/playback
  ini.c       INI file parsing
```

---

## 2. C module -> Rust module map

At the time of writing, the Rust tree is stubs only (`src/assets.rs`,
`src/palette.rs`, `src/sound.rs`, `src/main.rs`, `src/lib.rs`). The intended
mapping:

| C | Rust module(s) | Responsibility |
|---|---|---|
| `def.h` | `src/constants.rs` | `DIR_*`, entity counts/indices, `MWIDTH`/`MHEIGHT`, timing constants. |
| `main.c` | `src/game.rs`, `src/level.rs`, `src/main.rs`, `src/config.rs` | Flow (`game`, `mainprog`), level plans/data, `getlevch`, CLI/INI, RNG. |
| `digger.c` | `src/digger.rs` | Digger state, movement, firing, death sequence, bonus mode, emerald hit tests. |
| `monster.c` | `src/monster.rs` | Spawning, AI priority search, hobbin conversion, death, squash. |
| `bags.c` | `src/bag.rs` | Bag state, wobble/fall/gold timing, push chains, cleanup. |
| `drawing.c` | `src/render/field.rs`, `src/render/entity.rs`, `src/render/hud.rs` | Field bitmask + blob carving; `draw*` entity frames; text/lives/score HUD. |
| `sprite.c` | `src/sprite.rs` | Sprite records, positions/sizes, collision tests/lists. In the Rust design the screen save/restore buffers are replaced by layers (section 8). |
| `scores.c` | `src/scores.rs` | Scoring, extra lives, high-score table, initials. |
| `title.c` | `src/title.rs` | Title-screen animation and bitmap. |
| `input.c` | `src/input.rs` | Per-player direction/fire state, keyboard mapping, pause/escape. |
| `sound.c` + `newsnd.c` | `src/sound/effects.rs`, `src/sound/speaker.rs` | Effect/music state machines at 72.8 Hz; PC-speaker/PIT synthesis. |
| `win_vid.c` | `src/video.rs` | Index framebuffer, palettes, glyph/sprite blitting, present/scaling. |
| `win_snd.c` | `src/audio.rs` | `cpal` output stream and sample buffer. |
| `mkg.c` | `src/assets.rs` | Decode `vga.spr`/`vgatext.spr`/`vtitle.bmp` directly; no generated C arrays. |
| `record.c` | `src/record.rs` (roadmap) | DRF playback; also the best regression oracle. |
| `ini.c` | `src/config.rs` (roadmap) | Configuration persistence. |
| `win_sys.c` | `src/platform.rs` | Window/event loop, monotonic time, keyboard events. |

Keep the simulation modules free of windowing and audio types. The C code
achieves this with function pointers (`ginit`, `gputi`, ...); the Rust port
should use a `Renderer`/`AudioSink` boundary or, simpler, plain data
(`Framebuffer`, `Vec<SoundCommand>`) so the game logic is testable headlessly.

---

## 3. Coordinates, timing, and units

### Screen

* Logical screen: **320 x 200**. All game coordinates are in these units.
* Physical VGA back buffer: **640 x 400** (`win_vid.c:243-254` clears
  `640*400` bytes; `vgagetpix` samples an 8 x 2 physical block at `x*2,y*2`).
* Every sprite/character asset is authored at **2x**; the drawing routines
  double coordinates. One logical pixel is a 2x2 physical block.
* A "byte" in the C source is 4 logical pixels / 8 physical pixels; sprite
  widths (`sprwid`) are measured in those units.

### Field grid

* **15 x 10 cells** (`MWIDTH=15`, `MHEIGHT=10` in `def.h:63-65`).
* Cell size: **20 logical px wide, 18 logical px high**.
* Cell origin: `x = h*20 + 12`, `y = v*18 + 18` (`routines.txt:40`).
  Valid anchors: `x` 12..292, `y` 18..180.
* Horizontal digging granularity: **4 px** (`rx = (x-12)%20`, 5 columns/cell).
* Vertical digging granularity: **3 px** (`ry = (y-18)%18`, 6 rows/cell).
* Movement: digger/monster move 4 px horizontally or 3 px vertically per step;
  a falling bag moves **6 px** per update; a fireball moves **8 px**
  horizontally or **7 px** vertically per update.
* Sprite positions are x-aligned down to a multiple of 4
  (`movedrawspr`, `drawspr`: `x &= -4`, `sprite.c:71,101`).

### Directions

`def.h:34-38`: `DIR_NONE=-1`, `DIR_RIGHT=0`, `DIR_UP=2`, `DIR_LEFT=4`,
`DIR_DOWN=6`. The even values are used directly as animation indices in
`drawdigger` (direction / 2 selects the sprite row).

### Time

* PIT frequency: **1,193,181 Hz**; time is measured in PIT ticks
  (`routines.txt:46`).
* Default frame interval: `ftime = 80000` ticks (`main.c:859`) =
  **67.05 ms ≈ 14.92 fps**.
* `/S:n` sets `ftime = n*2000` ticks (`main.c:650-655`); accelerate/brake keys
  change it by ±10000 ticks, minimum 10000 (`input.c:295-301`).
* `newframe()` (`digger.c:81-127`) waits until `gethrt() >= curtime+ftime`,
  busy-calling `fillbuffer()` and `checkkeyb()`; it is called at the top of
  `dodigger()`, so it is the frame clock.
* `synchvid` optionally runs in 17094-tick steps (one 69.8 Hz refresh;
  `digger.c:87-98`).
* Gauntlet time is in ticks: `cgtime = gtime * 1193181` (`main.c:164`).
* Sound update unit: **0x4000 = 16384 ticks ≈ 72.8 Hz** (`routines.txt:49-50`,
  `sound.c:839-840`, `newsnd.c:249-255`).
* `soundint()` is called from the sample generator when `timercount`
  accumulates `timerrate` (normally 0x4000), not from the frame loop.

### Canonical unit recommendation

Use integer PIT ticks for the simulation (`u64` or `i64`), driven by a
monotonic clock converted as `ticks = nanos * 1193181 / 1_000_000_000`.
Do **not** copy `win_sys.c:110-131` literally: that path returns raw
`QueryPerformanceCounter` counts while `ftime` is interpreted in PIT ticks, and
the fallback multiplies `timeGetTime()` by 1193. The tick semantics above are
the authoritative ones.

---

## 4. The field bitmask model

`drawing.c:14` declares three arrays: `field1[150]`, `field2[150]` (saved per
player) and `field[150]` (current). One cell is a 16-bit word; `field` is
`Sint4` in C but only the low 16 bits are used.

### Bit layout

```
bit 15 14 13 12 | 11 10  9  8  7  6 |  5 |  4  3  2  1  0
     1  1  0x2000 1 |  r5 r4 r3 r2 r1 r0 |  x | c4 c3 c2 c1 c0
                    \_ 6 horizontal rows (3 px each) _/   \_ 5 vertical columns (4 px each) _/
```

| Bits | Mask | Meaning |
|---|---|---|
| 0..4 | `0x001f` | 5 vertical 4-px columns; **1 = still solid**, 0 = dug |
| 5 | `0x0020` | Constant/separator bit: 1 in all shipped cells; ignored by the "solid" test. |
| 6..11 | `0x0fc0` | 6 horizontal 3-px rows; **1 = still solid**, 0 = dug |
| 13 | `0x2000` | **Solid / not-a-path flag**: 1 = solid dirt, 0 = a path cell |
| 12,14,15 | `0xd000` | Unused, always 1 in shipped/masked values. |

"Solid cell" tests:

* `fieldclear` / `drawfield`: `(field & 0x2000) != 0` means solid.
* Bag tests and `drawfield` neighbour tests: `(field & 0xfdf) != 0xfdf`
  means **not** fully solid (bit 5 and the unused bits excluded). A pristine
  solid cell is `0xffff`, so `0xffff & 0x0fdf == 0x0fdf`.

### Level characters

`leveldat[8][10][15]` (`main.c:63-143`), selected by `levplan()`:

| Char | Meaning | `makefield` effect |
|---|---|---|
| `' '` (space) | solid dirt | remains `0xffff` |
| `H` | horizontal tunnel | `&= 0xdfe0` -> `0xdfe0` |
| `V` | vertical tunnel | `&= 0xd03f` -> `0xd03f` |
| `S` | start cell (both tunnels) | both masks -> `0xd020` |
| `C` | emerald | field stays solid `0xffff`; emerald lives in `emfield` |
| `B` | bag | field stays solid `0xffff`; bag lives in `bagdat` |

`makefield` (`drawing.c:47-63`) starts every cell at `-1` (0xffff), applies the
masks above, and copies the result into `field1`/`field2` for the current
player. `C` and `B` cells are solid until dug; the emerald/bag is an overlay.

`getlevch` (`main.c:145-150`) has a simultaneous-play special case: on plans 3
and 4, bottom row `y==9`, columns 6 and 8 return `'H'` when `diggers==2` and no
external level file is loaded. Preserve this.

### `eatfield` (`drawing.c:126-167`)

```
h=(x-12)/20, xr=((x-12)%20)/4, v=(y-18)/18, yr=((y-18)%18)/3
DIR_RIGHT: h++;                       field[v][h] &= ~(1<<xr);  if field[v][h]&0x1f  stop; else field[v][h] &= ~0x2000
DIR_LEFT:  xr--; if xr<0 {xr+=5;h--}; field[v][h] &= ~(1<<xr);  if field[v][h]&0x1f  stop; else field[v][h] &= ~0x2000
DIR_UP:    yr--; if yr<0 {yr+=6;v--}; field[v][h] &= ~(1<<(6+yr)); if field[v][h]&0xfc0 stop; else field[v][h] &= ~0x2000
DIR_DOWN:  v++;                       field[v][h] &= ~(1<<(6+yr)); if field[v][h]&0xfc0 stop; else field[v][h] &= ~0x2000
```

Note the asymmetry: moving right clears a column in the **next** cell, moving
left clears a column in the **current/previous** cell; moving down clears a row
in the cell below, moving up clears a row in the current/above cell. The width
step is `xr*4`, the height step is `yr*3`. `incpenalty()` is called once at
entry. Bounds are safe only because callers stop at `x==292`, `x==12`,
`y==180`, `y==18` before calling; do not "optimise" the boundary checks away.

### `drawfield` (`drawing.c:92-124`)

For every non-solid cell at `(x,y)` (`xp=x*20+12`, `yp=y*18+18`):

* If the six row bits are not all set (`(field & 0xfc0) != 0xfc0`), clear
  `0x0fc0` (`field &= 0xd03f`), then:
  `drawbottomblob(xp, yp-15)`, `drawbottomblob(xp, yp-12)`,
  `drawbottomblob(xp, yp-9)`, `drawbottomblob(xp, yp-6)`,
  `drawbottomblob(xp, yp-3)`, `drawtopblob(xp, yp+3)`.
* If the five column bits are not all set (`(field & 0x1f) != 0x1f`), clear
  `0x001f` (`field &= 0xdfe0`), then:
  `drawrightblob(xp-16, yp)`, `drawrightblob(xp-12, yp)`,
  `drawrightblob(xp-8, yp)`, `drawrightblob(xp-4, yp)`,
  `drawleftblob(xp+4, yp)`.
* If `x<14` and `(field[y][x+1] & 0x0fdf) != 0x0fdf`, `drawrightblob(xp,yp)`.
* If `y<9` and `(field[y+1][x] & 0x0fdf) != 0x0fdf`, `drawbottomblob(xp,yp)`.

`drawfield` both renders and **normalises** the field: once any bit of a group
is dug, the whole group is cleared. Reproduce the mutation if you intend to
save/restore or compare field words.

### Blob calls and geometry

| Function | Draw origin | `sprwid` x `sprhei` | Logical size | ch |
|---|---|---|---|---|
| `drawrightblob(x,y)` | `(x+16, y-1)` | 2 x 18 | 8 x 18 | 102 |
| `drawtopblob(x,y)` | `(x-4, y-6)` | 6 x 6 | 24 x 6 | 103 |
| `drawleftblob(x,y)` | `(x-8, y-1)` | 2 x 18 | 8 x 18 | 104 |
| `drawbottomblob(x,y)` | `(x-4, y+15)` | 6 x 6 | 24 x 6 | 105 |
| `drawsquareblob(x,y)` | `(x-4, y+17)` | 6 x 6 | 24 x 6 | 106 |
| `drawfurryblob(x,y)` | `(x-4, y+15)` | 6 x 8 | 24 x 8 | 107 |

All are mask-only sprites: the image is the zero buffer, and mask bit **0**
writes colour 0 (black tunnel) while mask bit **1** leaves the destination
untouched (`vgaputim`, `win_vid.c:1160-1190`). Tunnels are therefore black
holes carved into the dirt-pattern background.

### Background dirt tile

`drawbackg(l)` (`drawing.c:331-339`) tiles `ch = 93 + levplan()` (background
1..8) at `x = 0,20,...,300`, `y = 14,18,22,...,198`, size **5 x 4** logical
(40 x 8 physical). The pattern repeats every 20 logical px horizontally and
every 4 logical px vertically. It starts at y=14, i.e. 4 px above the first
field row at y=18. Do not clear the background to black: the dirt pattern is
part of the look and of fire collision (nonzero pixels).

### Emeralds (`emfield`)

`emfield[150]` is a **per-player bitmask**: bit 0 for player 0, bit 1 for
player 1 (`emmask = 1<<curplayer`, `digger.c:29,694,706`). `makeemfield` sets
or clears only the current player's bit, so both players' collected-emerald
state persists across switches.

* `'C'` cells set the bit; `drawemeralds` draws at `(x*20+12, y*18+21)`
  (note the +21 y offset, not +18) using ch 108.
* `hitemerald` (`digger.c:713-742`) adjusts `x`/`y` for direction and tests
  `r = rx` (horizontal) or `ry` (vertical) against
  `embox[8] = {8,12,12,9,16,12,6,9}`: `dir` and `dir+1` are the "draw" and
  "erase+hit" thresholds. `drawemerald` uses ch 108; `eraseemerald` uses ch
  109 (mask-only, clears to black).
* `killemerald(h,v)` clears the emerald **below** a bag: `(y+1)*15+x`.
* `hitemerald` is called by diggers (`updatedigger`) and hobbins (`monai`).

---

## 5. Entity/sprite system

### Entity indices (`def.h:42-61`)

```
SPRITES = 1 + 50 + 6 + 2 + 2 = 61
0        bonus
1..50    bags          (BAGS = 50)
51..56   monsters      (MONSTERS = 6)
57..58   fireballs     (FIREBALLS = DIGGERS = 2)
59..60   diggers       (DIGGERS = 2)
```

The `first[5]` collision array is indexed by type in drawing order:
`{bonus, bag, monster, fireball, digger}` (`sprite.c:257-259`).

### Sprite records (`sprite.c`)

Each active sprite has: `sprx, spry`, `sprwid` (width in 4-logical-px units),
`sprhei` (logical px), `sprch` (character code), `sprmov` (saved background
buffer), `sprenf` (visible), plus "new" values applied by `initspr`. The C code
saves the pixels under each sprite (`ggeti`) and restores them on erase/move;
the Rust compositor does not need this (section 8).

* `x` is rounded down to a multiple of 4 (`x & -4`).
* Collision box: `x .. x + sprwid*4 - 1`, `y .. y + sprhei - 1`
  (`bcollide`, `sprite.c:222-239`). Border fields are always 0 in this game.
* `bcollides(spr)` rebuilds `first[]`/`coll[]` for the **last drawn entity**,
  excluding itself. The Rust port should compute equivalent overlap lists
  (vectors are fine) preserving the type order and self-exclusion.

### Raw VGA sprite format (`vga.spr`)

* **107 records x 1024 bytes**. Each record is a **32 x 32 array of palette
  indices**, one byte per pixel, row-major.
* Pixel values **0..15** are opaque palette indices; **0xff is transparent**.
  For normal sprites the mask is derived from bit 7 (`byte & 0x80`), which is
  equivalent to `byte >= 0x80` because all colours are 0..15.
* The used sub-rectangle is the top-left **`(8*w)` x `(2*h)` physical
  pixels** (equivalently `(4*w)` x `h` logical pixels), where `w`/`h` are the
  `widths[]`/`heights[]` entries in `mkg.c:48-82`. Unused rows/columns are
  0xff.
* **Background records** (87..94) are special: logical size 5 x 4 (40 x 8
  physical). The first 32 px of each of the 8 rows come from bytes
  `row*32 + 0..31`; the remaining 8 px come from the second half of the record,
  bytes `512 + row*32 + 0..7` (i.e. rows 16..23).
* **Mask-only records** (blobs 80..85, and the mask half of ch 109/112) are not
  drawn as colour. For the blobs, bytes are `0x0f`/`0x00`; `0x0f` marks a
  protected pixel (mask 1, leave destination), `0x00` marks a pixel the blob
  clears (mask 0, write colour 0). For normal mask-only sprites the mask is
  bit 7 of the same record as the image.
* `edigger` (record 86) is used as the mask for the blank character codes 0 and
  31; it is not a visible sprite.

`mkg.c` also defines the *character-code* table (`vgatable[]`,
`mkg.c:286-417`). Character codes are not raw record numbers.

### Character code -> raw record map

`ch` is the number passed to `initspr(...)` / `drawmiscspr(...)`; the renderer
looks up `vgatable[ch*2]` (image) and `vgatable[ch*2+1]` (mask).

| ch | Meaning | Raw `vga.spr` records |
|---|---|---|
| 0 | blank / default | image zero, `edigger` mask |
| 1-3 | P1 digger right, anim 0-2, fire-ready | 25, 23, 21 |
| 4-6 | P1 digger right, anim 0-2, other state | 26, 24, 22 |
| 7-9 | P1 up, fire-ready | 37, 35, 33 |
| 10-12 | P1 up, other | 38, 36, 34 |
| 13-15 | P1 left, fire-ready | 31, 29, 27 |
| 16-18 | P1 left, other | 32, 30, 28 |
| 19-21 | P1 down, fire-ready | 43, 41, 39 |
| 22-24 | P1 down, other | 44, 42, 40 |
| 25 | P1 dead | 74 |
| 26-30 | P1 graves 1-5 | 73, 72, 71, 70, 69 |
| 31 | blank | zero image, `edigger` mask |
| 32-55 | P2 digger, same pattern as 1-24 but `bigger` palette | 49,47,45 / 50,48,46 / 61,59,57 / 62,60,58 / 55,53,51 / 56,54,52 / 67,65,63 / 68,66,64 |
| 56 | P2 dead | 75 |
| 57-61 | P2 graves 1-5 | 73, 72, 71, 70, 69 |
| 62-65 | bag: static, right, left, falling | 1, 2, 3, 4 |
| 66-68 | gold 1-3 | 5, 6, 7 |
| 69-71 | nobbin 1-3 | 10, 9, 8 |
| 72 | nobbin dead | 11 |
| 73-76 | right hobbin 1-3, dead | 12, 13, 14, 15 |
| 77-80 | left hobbin 1-3, dead | 16, 17, 18, 19 |
| 81 | bonus | 20 |
| 82-84 | fireball 1-3 | 95, 96, 97 |
| 85-87 | explosion 1-3 | 98, 99, 100 |
| 88-93 | duplicate fire/explosion (not used by current code) | 95..100 |
| 94-101 | background 1-8 | 87..94 |
| 102-107 | blobs: right, top, left, bottom, square, furry (mask-only) | 80, 82, 81, 83, 85, 84 |
| 108 | emerald | 0 |
| 109 | emerald erase (mask-only) | 0 |
| 110-113 | life right, life left, blank/extra-life, left-life | 76, 77, 78, 79 |
| 114-119 | P2 fire/explosion 1-3 (`b` variants) | 101..106 |

Entity dimensions used by the game:

| Entity | ch | `sprwid` x `sprhei` | Logical size |
|---|---|---|---|
| digger / bag / monster / bonus | as above | 4 x 15 | 16 x 15 |
| left hobbin death frame | 80 | 4 x 14 | 16 x 14 |
| fireball / explosion | 82-87, 114-119 | 2 x 8 | 8 x 8 |
| emerald | 108 | 4 x 10 | 16 x 10 |
| life icon | 110-113 | 4 x 12 | 16 x 12 |
| background tile | 94-101 | 5 x 4 | 20 x 4 |
| blob | 102-107 | 2x18 / 6x6 / 6x8 | 8x18 / 24x6 / 24x8 |
| text glyph | `vgatext.spr` | 12 x 12 | 24 x 24 physical |

### Digger animation formula

`drawdigger(n, t, x, y, can_fire)` (`drawing.c:362-383`):

```
nn = (n == 0) ? 0 : 31                  // digger slot 0 / 1
base = (t + (can_fire ? 0 : 1)) * 3 + 1 + nn
ch  = base + digspr[n]                  // digspr cycles 0,1,2,1
```

`t` is the direction (`0/2/4/6`). For `t=15` (dead) it uses `40 + nn - t`
(ch 25 or 56); for `t=10..15` it uses `40 + nn - t` (graves). `drawdigger`
increments `digspr` on every call and flips `digspd` at 0/2, so animation speed
is per draw call, not per unique frame. It clears `first[]` to `-1` when it
does not draw (e.g. `DIR_NONE`), preventing stale collision lists.

Monsters use the same 0,1,2,1 cycle (`monspr`) and:
`nobbin -> 69 + frame`, right hobbin -> `73 + frame`, left hobbin ->
`77 + frame`; death frames 72/76/80.

### Text (`vgatext.spr`, `win_vid.c:332-375`)

* **40 records x 1024 bytes**, each a 24 x 24 physical glyph (12 x 12 logical).
* Record order is fixed and **not ASCII**: `A..Z`, `0..9`, `.`, `:`, `_`,
  space (`mkg.c:599`). The runtime table `ascii2vga[0x5f]` maps ASCII-32 to
  these glyphs; lowercase letters map to uppercase.
* `outtext` advances x by **12 logical px** per character; glyphs are drawn at
  `(x*2, y*2)`.
* Text colour remap (`vgawrite`): glyph colour 10 (bright green) becomes
  12 (bright red) for `c==2` and 14 (bright yellow) for `c==3`; glyph colour
  12 (bright red) becomes 2 (green) for `c==1`, 4 (red) for `c==2`, and
  6 (brown) for `c==3`. So `c=1` is green text, `c=2` red, `c=3` yellow/brown.
* Unsupported characters blank a 24 x 24 area (the C code zeroes it).

### Palettes (`win_vid.c:26-93`)

Four 16-colour palettes are used; `gpal(0/1)` selects palette, `ginten(0/1)`
selects normal/intense. All values below are `(R,G,B)`.

| # | PAL1 normal | PAL1 intense |
|---|---|---|
| 0 | 0,0,0 | 0,0,0 |
| 1 | 0,0,128 | 0,0,255 |
| 2 | 0,128,0 | 0,255,0 |
| 3 | 0,128,128 | 0,255,255 |
| 4 | 128,0,0 | 255,0,0 |
| 5 | 128,0,128 | 255,0,255 |
| 6 | 128,64,0 | 255,128,0 |
| 7 | 128,128,128 | 192,192,192 |
| 8 | 64,64,64 | 128,128,128 |
| 9 | 0,0,255 | 128,128,255 |
| 10 | 0,255,0 | 128,255,128 |
| 11 | 0,255,255 | 128,255,255 |
| 12 | 255,0,0 | 255,128,128 |
| 13 | 255,0,255 | 255,128,255 |
| 14 | 255,255,0 | 255,255,128 |
| 15 | 255,255,255 | 255,255,255 |

PAL2 swaps red and green in the base colours (1=green, 2=red, 4=blue,
5=cyan, ...). It is used by the initials-screen palette flash
(`flashywait`, `scores.c:315-330`), not during normal play. The DOS build
loads a 256-colour table (`dospc.asm:1743`) whose 16-colour windows correspond
to the same four palettes; use the explicit Win32 tables above.

* During normal play: `gpal(0)`, `ginten(0)`.
* Bonus mode: `ginten(1)` (intense), and the first 20 frames of bonus mode
  toggle intensity every frame (`digger.c:174-204,630-646`).

`vtitle.bmp` is a 640 x 400, 4-bit, **RLE4-compressed** BMP whose 16-colour
palette is exactly PAL1 normal; blit it as palette indices (section 8).

---

## 6. Game rules and constants

### Lives and scoring

* Start with **3 lives** per digger (`initlives`, `digger.c:807-812`).
* Extra life at **20,000** points, then every 20,000 (`bonusscore = 20000`,
  `scores.c:39`).
* The extra-life condition is `score >= nextbs + n`, where `n` is the
  player index (`scores.c:176`). This is a preserved original bug: player 2
  gets the life one point later each time. Max 5 lives unless unlimited lives
  (`/U` or INI); in gauntlet mode the reward is **15 seconds** instead
  (`cgtime += 17897715`, `scores.c:180`).
* Score is clamped: above 999,999 it resets to 0 (`scores.c:167-168`).
* Every `addscore` calls `incpenalty()` three times (`scores.c:187-189`).

| Event | Points |
|---|---|
| Kill a monster with a fireball | 250 (`scorekill`) |
| Kill a monster with a bag in 2-sim | 125 to each player (`scorekill2`) |
| Emerald | 25 |
| Eighth consecutive emerald ("octave") | 250 |
| Gold | 500 |
| Bonus | 1000 |
| Eat a monster in bonus mode | `200 * msc`, `msc` doubles per monster and resets at bonus start |

The emerald "octave" counter: `emn` increments per emerald, wraps at 8 and
awards `scoreoctave`; `emocttime=9`; if a later emerald is collected after
`emocttime` reaches 0, `emn` resets to 0 first (`digger.c:429-444`).

### Monster spawning/difficulty (`monster.c:34-61`)

```
mongaptime   = 45 - levof10()*2        // frames between spawns
totalmonsters= levof10() + 5
maxmononscr  = 3 for level 1, 4 for levels 2-7, 5 for levels 8-10
nextmontime  = 10                      // initial delay
stime        = 5                       // per-monster spawn freeze
```

`levof10()` is the level capped at 10 (`main.c:559-564`).

Monsters spawn at cell (14,0), `x=292, y=18`, facing left. `chase` alternates
between diggers in simultaneous mode (`chase+curplayer`, then
`(chase+1)%diggers`). If the chased digger is dead, the other is used.

### Hobbin/nobbin conversion (`monster.c:86-105,154-160,313-314`)

* A nobbin becomes a hobbin when `hnt > 10 - levof10()` (checked per frame in
  `domonsters`), then `hnt=0`.
* A hobbin reverts to a nobbin when `hnt > 30 + levof10()*2` (checked at cell
  alignment in `monai`), then `hnt=0`.
* `hnt` increments for hobbins each move (capped at 100), and for nobbins on
  monster-monster collisions and when stuck on a bag.
* Hobbins ignore `fieldclear` and move in the highest-priority direction, dig
  tunnels (`eatfield` + blobs) and can eat emeralds/bags.
* Nobbins move only through open field cells (`fieldclear`).

`fieldclear` (`monster.c:455-483`):

```
RIGHT: x<14 && next(x+1,y) not solid && (next bit0 ==0 || cur bit4 ==0)
LEFT : x>0  && next(x-1,y) not solid && (next bit4 ==0 || cur bit0 ==0)
UP   : y>0  && next(x,y-1) not solid && (next bit11==0 || cur bit6 ==0)
DOWN : y<9  && next(x,y+1) not solid && (next bit6 ==0 || cur bit11==0)
```

### Bags and gold (`bags.c`)

* `goldtime = 150 - levof10()*10` frames (140 at level 1, 50 at level 10).
* Initial state: `wt=15`, `wobbling=FALSE`, `unfallen=TRUE`, `dir=NONE`,
  `gt=0`, `fallh=0`, anchored at `(x*20+12, y*18+18)`.
* A bag whose supporting cell below has been dug starts wobbling (`wt=15`),
  unless a digger is moving vertically underneath it (`checkdiggerunderbag`).
* Wobble animation: `wblanim[4]={2,0,1,0}` (left, static, right, static) for
  even `wt%8`.
* When `wt` reaches 0, `dir=DOWN`, `soundfall()`. A bag falls **6 px per
  update**; `fallh` increments when `yr==0`.
* A bag breaks (becomes gold) only if it fell with `fallh > 1`; otherwise it
  just lands. On landing it removes bags it collides with.
* Gold timing: `gt` increments each frame; frames 1, 3, 5 draw gold frames
  4, 5, 6 and play `soundbreak` at 1. At `gt == goldtime` the bag is removed.
  If the cell below is open and `gt < goldtime-10`, `gt` jumps to
  `goldtime-10` (gold over a hole disappears sooner).
* Pushing:
  * Horizontal push advances the bag 4 px, sets `pushcount=1`, and if the bag
    hits a digger or monster the push is reverted (the bag stays put and
    `dir=reversedir(dir)`).
  * A falling bag pushed horizontally cannot be pushed: it kills diggers whose
    y is at/below the bag and squashes monsters, then returns success.
  * Vertical push (`pushudbags`) only succeeds for already-broken gold (`gt!=0`)
    which is collected.
  * The digger pays one extra frame (`bagtime++`) per horizontal push; while
    `bagtime!=0` the digger redraws in `mdir` and does not move.
* Digger death by bag: `killdigger(n, 1, bag)`. A falling bag's horizontal push
  kills diggers below it; a falling bag also squashes monsters
  (`squashmonsters`, only those with `y >= bagy`).
* `cleanupbags` (`bags.c:73-89`), run after a death/level end, removes any bag
  that is at `(h=7, v=9)` (single-player start cell), moving (`xr!=0||yr!=0`),
  broken (`gt!=0`), has fallen (`fallh!=0`) or is wobbling. This is the rule
  that prevents "half-broken" bags from persisting across a death. Note the
  hard-coded `(7,9)` start cell; in simultaneous mode the other start cells
  are not covered.
* `getnmovingbags` (`bags.c:402-410`) counts `exist && gt<10 && (gt!=0 ||
  wobbling)`; used to decide when the post-death action has settled.

### Bonus mode (`digger.c:624-646`)

* `createbonus` places sprite 0 at `(292,18)` after all monsters have spawned
  and `nextmontime` has expired (`monster.c:80-85`), once per level.
* Collecting it gives 1000 points and starts bonus mode:
  `bonustimeleft = 250 - levof10()*20` (230 at level 1, 50 at level 10),
  `startbonustimeleft = 20`, `msc=1`.
* During the first 20 frames the screen flashes (intensity every frame,
  `soundbonus`), then `music(0)` (William Tell) plays. When the timer expires,
  `endbonusmode`, `soundbonusoff`, `music(1)` (Popcorn).
* In bonus mode monsters flee (AI priority inversion) and can be eaten by
  collision: `killmon` + `sceatm` per digger. Killing a monster increments
  `totalmonsters`, so replacements keep spawning.
* If all diggers die in bonus mode, the mode ends immediately.

### Fire (`digger.c:207-372`)

* Recharge: `levof10()*3 + 60` frames.
* Origin offsets by firing direction:
  right `(x+8,y+4)`, up `(x+4,y)`, left `(x,y+4)`, down `(x+4,y+8)`.
* Movement per frame: right +8, left -8, up -7, down +7.
* Explodes on: screen limits (`fx>296`, `fx<16`, `fy<15`, `fy>183`), nonzero
  terrain pixel ahead (`ggetpix`/`vgagetpix`), or any sprite collision.
* Kills all colliding monsters (250 each), can kill the **other** digger
  (stage-3 death; friendly fire), and exploding fireballs chain-explode each
  other. An explosion runs frames 1,2,3 then `killfire`.

### Death sequence (`digger.c:503-622`)

| Stage | Behaviour |
|---|---|
| 1 | Bag death. Follow the bag down (`y = bagy(bag)+6` while above it), draw frame 15; when `getbagdir(bag)+1==0` (bag gone or `DIR_NONE`), play `soundddie`, `deathtime=5`, stage 2, `y-=6`. |
| 2 | Wait `deathtime`; draw frames `14-deathani` (14,13,12,11,10), 2 frames each; `music(2)` on ani 0; if ani 0, kill colliding monsters without score; after ani 4 -> stage 4 with `deathtime=60` if music is on or `diggers>1`, else 10. |
| 3 | Immediate transition to stage 5 (monster/fire death). |
| 5 | Bounce: draw frame 15 at `y - deatharc[ani]`, `deatharc={3,5,6,6,5,3,0}`, 7 frames; `soundddie` on ani 1; `musicoff` on ani 6 if no digger is alive; then `deathtime=5`, ani 0, stage 2. |
| 4 | Wait, set `dead=TRUE`. The scan `for (i=0;i<diggers;i++)` looks at slots `0..diggers-1`, not `curplayer..curplayer+diggers`. With one digger per screen this is effectively "round over": call `setdead(TRUE)` and let `game()` decrement the life and switch/restart. In simultaneous mode (`diggers==2`) it checks both active slots; if the other digger is alive, decrement this digger's lives and respawn immediately if lives remain. |

* `killdigger` is ignored while invincible, and while `deathstage` is 2..4.
* Respawn: place at start cell, facing left/right by digger index,
  `invin=TRUE`, `ivt=50`, clear fire, `music(0 if bonusmode else 1)`.
* Invincibility blinks: each draw decrements `ivt`; while `ivt%10 < 5` the
  sprite is erased after drawing (`drawdig`, `digger.c:131-142`).
* `isalive()` checks only the active slots (`curplayer..curplayer+diggers`).

### Level completion and flow

* `checklevdone` (`main.c:532-538`): a level is complete when
  `countem()==0 || monleft()==0`, **and** at least one digger is alive.
  Killing all monsters ends the level even with emeralds left.
* On level end: `erasediggers`, `musicoff`, then up to 20 extra frames while
  bags are moving (or until the penalty stays low), `soundstop`, kill fire,
  erase bonus, `cleanupbags`, `savefield`, `erasemonsters`.
* If the level was completed (or all emeralds gone): any dead digger with lives
  is decremented, level++ (capped at 1000), `initlevel`.
* If all diggers are dead: decrement lives and redraw; game over when
  `alldead && getalllives()==0 && !gauntlet`, or gauntlet time expires.
* In alternating mode a "round" is one life: the middle loop ends as soon as
  `alldead` is set (one digger per screen), the life is decremented, and
  `game()` switches `curplayer` if the other player still has lives. Level
  progress is kept per player in `gamedat[0/1]`, so turns resume on the level
  each player reached. In simultaneous mode a round continues while either
  digger is alive.
* A level redraw (`levnotdrawn`) rebuilds the static screen; after a death on
  the same level only `initchars` runs, preserving the carved background.

### Level plan (`main.c:551-557`)

```
levplan() = level                       if level <= 8
          = (level & 3) + 5             if level > 8
```

This yields: 1,2,3,4,5,6,7,8, then 6,7,8, then `(5,6,7,8)` repeated 247 times
(levels 12..999), then level 1000 maps to 5. `level` is capped at 1000
(`main.c:275-276`). The plan selects both the level data (`leveldat[l-1]`) and
the background tile (`ch=93+l`).

### RNG (`main.c:819-825`)

```
randv = randv*0x15a4e35 + 1
return (randv & 0x7fffffff) % n
```

Use **32-bit wrapping arithmetic** (`randv` is a 32-bit signed `long` in the
original DOS build; signed overflow wraps in practice). `randv` is seeded from
the low-resolution timer at the start of each round (`randv=getlrt()`), and
recorded/restored for DRF playback. A separate PRNG (`randvs`,
`sound.c:71-77`) is used by sound so that toggling sound does not change
gameplay. Reproduce both exactly.

---

## 7. Sound architecture

### What the C code does

The C code emulates the PC's 8253 PIT and speaker in two layers:

1. **High level (`sound.c`)**: every effect/music has a state machine advanced
   by `soundint()` at **72.8 Hz**. Each writes `t0val` (timer 0 / int8 period)
   and/or `t2val` (timer 2 / tone period), plus `wavetype`, `musvol`,
   `pulsewidth`. The update order inside `soundint` is fixed and matters when
   several effects overlap (`sound.c:96-124`):
   `music -> emerald -> wobble -> ddie -> break -> gold -> em -> explode ->
   fire -> eatm -> fall -> 1up -> bonus`.
   The last writer of `t2val` wins; `t0val`/`t2val` are reset to
   `0x7d00`/`40` at the start of each update.
2. **Low level (`newsnd.c`)**: `getsample1()` emulates the PIT/speaker
   sample-by-sample, averaging the speaker state over each output sample. The
   timer 2 reload is **halved** (`s1settimer2`, `newsnd.c:124-130,152-159`),
   so the audible tone is `f = 1193181 / t2val`; the music/effects note values
   are therefore PIT divisors (e.g. `0x8e8 = 2280` -> 523 Hz, C5). `t2val=40`
   is the "speaker off" sentinel (it reloads at `rate/2` to avoid aliasing).
   `t0val=0x7d00` is "rest/idle".
3. `wavetype`/`spkrmode`:
   * `0`: speaker not driven by int8; the timer-2 tone is output continuously
     (the effect-only path via `setsoundt2()`).
   * `1`: music pulse-width mode: each int8 event starts a pulse whose length
     is `pwlut[pulsewidth]` (`pwlut[i]=(16+i*18)>>2`,
     `newsnd.c:78-79`); `pulsewidth = musvol*volume`. This is how volume
     envelopes are produced on a 1-bit speaker.
   * `2`: int8 gate/toggle mode, used when combining timer 0 and timer 2
     (e.g. the level jingle). `setspkrt2()`/`setsoundt2()` connect the speaker
     to timer 2.

`soundint()` is called from the sample generator when
`timercount += timerrate` overflows (normally `timerrate=0x4000`), not from the
game frame loop. `soundstop()` resets every effect; `soundlevdone()` blocks the
game loop until the 11-note jingle finishes, pumping `fillbuffer` and
`checkkeyb`.

### Music and effect tables

* Tunes (`music(tune)`, `sound.c:624-654`):
  * 0 = William Tell Overture (bonus), tempo multiplier 3, `bonusjingle[321]`.
  * 1 = Popcorn (normal), multiplier 6, `backgjingle[291]`, note width 12.
  * 2 = Chopin's Funeral March (death), multiplier 10, `dirge[]`.
  * All tables terminate with `0x7d64`.
  * Each tune has envelope parameters (max volume, attack, sustain, decay,
    release) applied in `musicupdate` (`sound.c:712-777`).
* Effect constants live in `sound.c`: `emfreqs[8] = {0x8e8,0x7f0,0x712,
  0x6ac,0x5f2,0x54c,0x4b8,0x474}`, `newlevjingle[11] = {0x8e8,0x712,0x5f2,
  0x7f0,0x6ac,0x54c,0x712,0x5f2,0x4b8,0x474,0x474}`.
* Effect behaviours (values are `t2val` / durations): fall 1000 (+50 every
  other update), break 15000 (3 updates), wobble 0x7d0/0x9c4/0xbb8 cycle,
  fire 500 growing by /55 (jittered with the sound PRNG, capped 30000),
  explode 1500 decaying by 7/8 over 10 updates, bonus 0x4ce/0x5e9 alternating,
  em 1000 one-shot, emerald 7 cycles of the note, gold 500/4000 converging over
  30 updates, eatm 2000 decaying over 3x20 updates, ddie 20000 ramping down
  then up (max 30000), 1up duration 96 with pitch `(dur<<2)+600`, level jingle
  20 updates per note with `t0val=t2val+35`.

### Recommended Rust design (command-based)

Do not port the function-pointer soup or the C global flags. Use explicit
commands and an isolated engine:

```
enum SoundCommand {
    Music(Tune),                 // 0 bonus, 1 normal, 2 death
    StopMusic,
    Start(Effect, Slot),         // Slot: 0..1 for fire/explosion, 0 for others
    Stop(Effect, Slot),
    StopAll,
    LevelJingle,
    Pause, Resume,
    SetSoundEnabled(bool), SetMusicEnabled(bool),
}
```

* The simulation emits commands at the same call sites as the C
  `sound*`/`music` functions. A `SoundEngine` owns one state machine per
  effect, exactly matching the C fields (`value`, `duration`, `n`, flags) and
  implements the same 72.8 Hz update order.
* Drive the engine from the audio path, not the game loop. Compute the number
  of 0x4000-tick sound updates per audio callback from the sample rate:
  `updates = samples * 1193181 / (16384 * sample_rate)`, with a remainder
  accumulator, or tick it from the game loop using the same accumulator. Keep
  the update order and constants above; that is what makes the sound
  recognisably Digger.
* Synthesise the speaker signal from `(t0val, t2val, wavetype, pulsewidth,
  t2sw)` rather than copying the 12-case `getsample1` state machine verbatim.
  Two clean equivalents:
  * timer-2 tone: square wave at `PIT/t2val`, gated by `t2sw`;
  * int8 pulse/toggle: square wave at `PIT/t0val` with duty cycle
    `pwlut[pulsewidth] / t0val` (music), or a gate for wavetype 2.
  For the first playable version it is acceptable to render the tone directly
  and ignore the exact 1-bit mixing quirks, but keep the state-machine timing
  and the `t2val=40` / `t0val=0x7d00` sentinels.
* Use a separate PRNG seed for sound jitter. Never touch the game's `randv`
  from audio code; audio runs on another thread.
* `cpal` output: mono, unsigned 8-bit-equivalent or f32; the C output is
  8-bit unsigned (`samp`, 0..255). Clamp/convert; keep a small latency buffer.
* High-level toggles (`soundflag`, `musicflag`) map to the `Set*Enabled`
  commands; do not let them alter gameplay.

---

## 8. Rendering architecture for the Rust port

The C renderer is a single 640 x 400 8-bit framebuffer with save/restore
buffers around each sprite (`ggeti`/`gputi`) and a recursive redraw list. The
port should replace this with layers and keep only the visible behaviour.

### Recommended pipeline

1. **`Framebuffer`**: `Vec<u8>` of **640 x 400 palette indices** (the physical
   resolution). Logical coordinates are multiplied by 2 on draw. This matches
   the native 2x art and avoids resampling.
2. **`bg` static layer** (640 x 400 indices):
   * On level start (`levnotdrawn`): fill with black, tile the dirt background
     (`ch=93+levplan()`, 20x4 logical tiles at x=0..300, y=14..198), then apply
     the level field (`drawfield` equivalent) to carve tunnels to black, then
     draw emeralds for the current player (`emfield`).
   * On a player switch or new level: rebuild `bg` from scratch. On a death,
     keep it: carved dirt is permanent.
   * Incremental carving: when a digger/monster/bag calls `eatfield` or a blob
     draw, apply the blob mask directly to `bg` (mask bit 0 -> `bg=0`).
     `drawemerald`/`eraseemerald` also mutate `bg`. There is no need to
     restore sprite backgrounds.
3. **`frame`**: each tick, `frame.copy_from_slice(&bg)`, then blit every
   active entity from its sprite record in the C draw order (see below), then
   draw HUD text/lives. The sprite layer is fully redrawn every tick; there are
   at most 61 entities, so this is cheap.
4. **Present**: apply the selected palette (PAL1/PAL2, normal/intense) at
   present time, converting indices to 32-bit RGB, then nearest-neighbour
   integer-scale into the window. Pick
   `scale = max(1, min(window_w/640, window_h/400))`, centre, letterbox.
   Never use bilinear filtering.

### Entity draw order

C draws entities as they are updated: `dodigger` (digger + fireball) ->
`domonsters` -> `dobags`; at level start it draws bags then emeralds then
diggers/monsters. The C sprite system redraws overlapping sprites recursively,
so exact overlap results depend on draw order. For a deterministic port use
the entity index order (`0` bonus, `1..50` bags, `51..56` monsters, `57..58`
fireballs, `59..60` diggers) or the update order above, and document the
choice. Overlaps are visually rare; collisions are computed from records, not
from pixels.

### Sprite records in Rust

```
struct SpriteRecord {
    ch: u8,          // character code -> decoded image/mask
    x: i32, y: i32,  // logical; x is aligned down to 4
    w: i32, h: i32,  // logical dimensions
    enabled: bool,
}
```

Decoded assets:

```
struct SpriteImage { w: u8, h: u8, pixels: Vec<u8>, mask: Vec<bool> }
```

* Decode `vga.spr` once: 107 records; used rect `(8*w) x (2*h)` physical
  pixels; 0xff transparent; background special case; mask-only records as
  described in section 5.
* For normal sprites, blit `pixels` where `mask` is false (mask bit 0) and
  leave `frame` otherwise. Note that palette index 0 is a *colour* (black), not
  transparency.
* `drawmiscspr` (blobs, emeralds, lives) becomes either a `bg` operation or a
  direct frame blit; blobs are "write 0 where mask bit is 0".

### Text/HUD

Decode the 40 `vgatext.spr` glyphs once. `outtext(s,x,y,c)` draws each glyph at
`(x*2, y*2)`, remaps colours as in section 5, and advances x by 12. HUD text
is drawn after sprites so that score/lives/pause messages are always legible
(this is a deliberate deviation from the C order, whose top line has no
gameplay sprites except during the title/demo).

### `vgagetpix` equivalent

`vgagetpix(x,y)` (`win_vid.c:310-329`) samples an **8 x 2 physical** block
starting at `(2x,2y)` (i.e. 4 logical px wide, 1 logical px tall), ORs the two
rows together into an 8-bit mask of nonzero pixels (`0xff` outside the screen),
then masks with `0xee`. Fire uses it to test terrain ahead. In the port, test
`bg` at those physical pixels (terrain, dirt and emeralds are in `bg`); sprite
collisions are handled separately through the collision lists, so consulting
`bg` is sufficient and avoids an ordering dependency on the current frame's
sprite composite. Replicate the `0xee`, `0xc0` (up) and `0x03` (down) masks
exactly.

### Title screen

`vtitle.bmp` is 640 x 400 RLE4 with PAL1-normal indices. Decode it to an index
buffer and present it as the background. Reproduce the animation in
`mainprog` (`main.c:329-452`): frames 50..77 slide monster 0 from x=292 to 184
(4 px/frame), then it faces right; frames 90..117 monster 1; 130..157 digger;
178 bag/gold; 198 emerald; 218 bonus. Text labels are drawn at the frames given
in that loop (`NOBBIN` at 83, `HOBBIN` 123, `DIGGER` 163, `GOLD` 183,
`EMERALD` 203, `BONUS` 223). The loop resets `frame` above 250. Start on any
key except ESC/N/n; ESC/N/n toggles one/two players (not in gauntlet or
2-simultaneous).

---

## 9. Behaviours that look like bugs but must be preserved

These affect gameplay, timing, or the recorded-input format. Do not fix them
without an explicit design decision and a conformance test.

1. **Extra life `nextbs+n`** (`scores.c:176`): player 2's threshold is one
   point higher each time due to `+ n`. Keep it.
2. **Score wrap** (`scores.c:167-168`): `score > 999999` becomes 0, and the
   extra-life check also requires `score < 1000000`.
3. **Level plan** (`main.c:551-557`): 1..8, then 6,7,8, then
   `(5,6,7,8)` x247, level 1000 -> 5. `level` is capped at 1000.
4. **`getlevch` simultaneous-play special case** (`main.c:145-150`):
   plans 3/4, row 9, columns 6/8 -> `'H'` when `diggers==2` and no level file.
5. **`incmont` starts at 1** (`monster.c:547-554`): `for (m=1;m<n;m++)`, so
   monster slot 0 never receives the penalty. Also `n` is capped at 6.
6. **Penalty-driven timing**: every `addscore` adds 3 to `penalty`; many draw/
   push/death paths call `incpenalty`. After `dodigger/domonsters/dobags`, if
   `penalty>8`, `incmont(penalty-8)` delays monsters. If the Rust renderer is
   cheaper, still perform the same `incpenalty` calls; otherwise monster timing
   diverges from the original and from DRF recordings.
7. **Bag cleanup** (`bags.c:73-89`): only `(h=7,v=9)`, moving, broken, fallen,
   or wobbling bags are removed after a death. `(7,9)` is the single-player
   start cell; in simultaneous play the starts are `(8,9)` and `(6,9)`, so a
   bag sitting on a start cell there is not removed by this test. Keep the
   exact test.
8. **`mondie` stage 1** (`monster.c:428-441`): the check uses
   `bagy(bag)+6 > y`, but the assignment is `y = bagy(bag)` (without +6).
   Preserve the resulting vertical jump.
9. **Left-hobbin death frame** uses `h=14` (`drawing.c:225`), one row shorter
   than every other death frame. The raw asset (`mkg.c` heights index 19) also
   says 14.
10. **`mongold` / no time penalty** (`monster.c:373-374`): if a monster breaks
    a bag with no digger collision, `t=0`.
11. **Monster-monster collision `kludge`** (`monster.c:337-348`): the penalty
    count differs for DRFs recorded before 19981125 (`record.c:83-85`). Keep
    the playback switch; for normal play use the non-kludge path.
12. **Fire friendly-fire** (`digger.c:282-290`): a fireball can kill the other
    digger unless that digger is invincible/alive checks fail. Keep it.
13. **`drawfield` mutates the field** while drawing (normalises each bit group
    once any bit is dug). If you compare field words or save/restore them,
    reproduce this.
14. **Bit 5** (`0x20`) is a constant/separator bit: it is 1 in every shipped
    cell and is deliberately excluded from the solid test `0x0fdf`. Do not
    repurpose it.
15. **`eatfield` asymmetry** (section 4): right clears the next cell's column;
    left clears after a wrap; up/down clear opposite rows. Preserve index
    arithmetic exactly.
16. **`drawdigger` advances animation per call**, including the bag-push
    branch and re-draw after a failed push; it is not a per-frame clock.
17. **`drawlives` shows `lives-1` icons** (`drawing.c:395`): the current life is
    not displayed; with more than 4 remaining it shows "Xn". Keep the layout
    (left player at x=80..140, right player at 224..164, time text at 124).
18. **`initdigger` uses the absolute digger index** for facing
    (`dig==0 ? RIGHT : LEFT`), so alternating player 2 starts facing left.
19. **`cleanupbags` and the start cell** are single-player-centric: the test is
    the literal `(h==7 && v==9)`, while simultaneous starts are `(8,9)`/`(6,9)`.
    Keep the literal unless you deliberately define new cleanup semantics.
20. **Level completion on `monleft()==0`** (`main.c:534`): killing all monsters
    finishes the level even if emeralds remain.
21. **Death on the last emerald**: level-completion life decrement only applies
    to dead diggers with lives (`main.c:270-272`), and `checklevdone` requires
    `isalive()`; preserve the ordering.
22. **`bonusvisible=TRUE` at game start** (`main.c:178`) without drawing a
    bonus: `erasebonus` is a no-op because the sprite is not enabled.
23. **`drawfire` player-2 offset**: `nn = (n==0) ? 0 : 32`, so P2 uses
    ch 114..119; ch 88..93 duplicate P1 fire/explosion and are unused.
24. **`soundlevdone` blocks the game loop** until the jingle ends (or escape).
    If you make it asynchronous, level-end timing changes relative to the
    original; keep it as a timed state or document the deviation.

---

## 10. Getting oriented: checklist and roadmap

### New-contributor checklist

1. Read `ref/remastered/routines.txt` first (terminology, units, module map),
   then `def.h` and `digsrc.txt`.
2. Read `main.c:156-301` (`game`) and `main.c:329-452` (`mainprog`) to see the
   outer loops and title flow.
3. Read `drawing.c:47-167` (field/make/draw/eat) and `digger.c:383-493`
   (`updatedigger`) together; this is the core interaction loop.
4. Read `monster.c:143-426` (`monai`) and `bags.c:136-340` (bag update/push)
   next.
5. Decode the assets before writing gameplay:
   * `vga.spr`: verify record 0 (emerald), 1-7 (bags/gold), 8-19
     (monsters), 20 (bonus), 21-75 (digger frames/graves), 76-79 (life
     icons), 80-85 (blob masks), 86 (edigger mask), 87-94 (backgrounds),
     95-106 (fire/explosion).
   * `vgatext.spr`: verify record 0 is `A` (not ASCII 65's record).
   * `vtitle.bmp`: verify 640x400, 4-bit RLE4, PAL1-normal palette.
6. Build a headless harness: `Framebuffer` + `Renderer` + fixed tick. Dump
   PPM/PNG frames for visual comparison against the C build or a known
   screenshot.
7. Add `randv` and a frame counter to the harness; DRF playback is the best
   end-to-end oracle (`record.c`, `drf.txt`).
8. Only then wire `winit`/`softbuffer`/`cpal`; keep the simulation testable
   without them (as `lib.rs` intends).

### Suggested implementation order

1. Constants, level data, `levplan`/`getlevch`, RNG.
2. Asset decoding + palette + framebuffer + present.
3. Field model, background build, `drawfield`, `eatfield`, blob masks.
4. Sprite records + collision lists + HUD text.
5. Digger movement, digging, fire, emerald collection.
6. Monsters (spawn/AI/hobbin/death), bags (wobble/fall/push/gold), bonus mode.
7. Death/respawn, scoring/extra lives, level flow, title screen.
8. Input state machine (`readdir`/`getdir`), pause, speed keys.
9. Sound engine (state machines first, synthesis second).
10. Two-player alternating/simultaneous, gauntlet, level files, high scores.
11. DRF record/playback and regression tests.
12. Config/INI, key redefinition, CGA mode (optional), gamepad (optional).

### Known gaps in the current Rust tree

* At the time of writing, `src/` is stubs only: no simulation, renderer, asset
  decoding, or audio.
* `Cargo.toml` already selects `winit 0.30`, `softbuffer 0.4`, `cpal 0.18`,
  `edition 2021`, `rust-version 1.86`, `license = "GPL-2.0-or-later"`. The
  dependency versions are pinned; verify they build on the target platforms.
* No test fixtures. Use the reference build's screenshots and DRF files as
  fixtures; keep them out of the shipped binary if licensing is unclear.
* Decide explicitly whether CGA (`cga.spr`, `cgatext.spr`, different palettes)
  is in scope. The VGA path is the primary target; CGA is a compatibility
  feature at best.
* Decide whether to reproduce the C's exact sprite-overlap draw order or use a
  stable entity order. Document the choice and compare screenshots.

### Open questions / ambiguities found while reading the reference

* `win_sys.c:110-131` (Remastered) returns raw `QueryPerformanceCounter`
  counts from `gethrt()` while `ftime` is documented in PIT ticks; the Windows
  snapshot's timing path is internally inconsistent. The Rust port should use
  PIT ticks as the canonical unit (section 3) and not copy this code.
* `routines.txt` says hobbins "decrement" `hnt`, but the code only increments it
  (capped at 100) and resets it on state changes (`monster.c:313-314,375-382`).
  Trust the code; the prose is stale.
* `routines.txt` describes `wavetype` 1 as "toggle in int8" and 2 as "pulse in
  int8", but `getsample1` (`newsnd.c:226-236`) makes spkrmode 1 the
  pulse-width branch and spkrmode 2 the toggle branch. The code is consistent
  with the music envelope using `pulsewidth`, so trust the code. This only
  matters if you emulate the 1-bit mixer exactly.
* `mkg.c`'s `vgatable` contains duplicate entries at ch 88..93
  (`vgafire1..vgaexp3` again). Current drawing code never uses them; treat
  them as historical aliases.
* The GPL grant and `digsrc.txt:21-25`'s "no money-making / relinquish rights"
  wording coexist in the same release. This port takes the conservative
  position described in `assets/ATTRIBUTION.md`; the ambiguity is a rights
  question, not a code question.
