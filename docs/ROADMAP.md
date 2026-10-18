# Roadmap

Status of the port. Keep this in sync with `CLAUDE.md`.

## M1 — Foundation

- [x] Cargo project, module skeleton, GPL license, README/CLAUDE/docs
- [x] Original assets embedded (`vga.spr`, `vgatext.spr`, `vtitle.bmp`)
- [x] Indexed 640x400 renderer skeleton (background, blobs, text, palette)
- [x] Sprite record + collision port (`sprite.c`)
- [x] Keyboard input bridge and original direction latches
- [x] Threading model: winit UI, game thread, cpal audio, shared frame
- [x] Asset decoder: all 107 sprites, masks/backgrounds, 40 glyphs, BI_RLE4 title
- [x] Palette module + text colour remap
- [x] PC-speaker emulator + `sound.c` state machine (validated sample-for-sample against C)
- [x] First successful `cargo run`

## M2 — Playable single player

- [x] Title screen (vtitle + animated sprite showcase + high-score table)
- [x] Level render: dirt texture, pre-dug tunnels, emeralds, bags
- [x] Digger movement, digging, emerald collection
- [x] Fireballs and explosions
- [x] Nobbin/Hobbin AI, monster creation/bonus
- [x] Bag wobble/fall/gold/push, bag kills
- [x] Death animation, lives, level completion, next level (verified headlessly)
- [x] Score, extra lives, HUD (score/lives)
- [x] Game over + high-score initials + persistence
- [x] Sound effects + background/bonus/dirge music

## M3 — Two player and parity

- [x] Alternating two-player mode (N on title)
- [x] Simultaneous two-player mode (S on title; headless fuzz-tested)
- [ ] Gauntlet mode (`/G`) and time display
- [x] Speed control, music/sound toggles, pause
- [ ] Compare behaviour against Digger Remastered (speed, feel, quirks)
- [ ] Level-file (`.lvl`) support, optional
- [ ] Demo/record (`.drf`) playback, optional (archival)

## M4 — Polish

- [x] Fullscreen (F11); remembered window size still todo
- [ ] Config file / command-line options
- [x] Player-1 gamepad support (left stick/D-pad/fire/Start)
- [ ] Optional CRT-style shader? (not planned; software renderer kept simple)
- [x] Packaging scripts + CI release workflow (macOS universal, Windows x86_64; Linux via CI)
- [x] CI: fmt, clippy, tests on three platforms
- [ ] Accessibility/readability: optional larger HUD? (off by default)

## Known fidelity questions

- Exact default frame rate: Remastered uses `ftime=80000`; confirm against
  original DOS speed.
- Original sound device selection used `getsample1` by default on Windows;
  we target that accurate emulator.
- Title screen composition: Windows Remastered blits `vtitle.bmp`; the DOS
  build decodes `vgatitledat`. We use the bitmap.
- Whether to expose Digger Remastered's original control defaults (P2
  S/W/A/Z) in addition to the modern WASD mapping.
