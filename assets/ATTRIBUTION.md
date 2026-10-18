# Attribution and asset provenance

This project is a cross-platform Rust remake of **Digger**, the 1983 game by
**Windmill Software**. Its behaviour and artwork are derived from **Digger
Remastered** by **Andrew Jenner** (1998-2004), which is the reference tree in
`ref/remastered/`. This file records where the bundled assets came from, what
their licensing says, and the position this project takes. It is a provenance
note, not legal advice.

## Bundled files

| File | Size | What it is | Source in the reference tree |
|---|---|---|---|
| `assets/vga.spr` | 109,568 bytes | VGA sprite data: **107 records x 1024 bytes**, each a 32 x 32 array of palette indices (see `docs/PORTING_NOTES.md` section 5). | `ref/remastered/vga.spr` (file date 2001-08-24) |
| `assets/vgatext.spr` | 40,960 bytes | VGA font: **40 records x 1024 bytes**, each a 24 x 24 glyph (A-Z, 0-9, `.`, `:`, `_`, space). | `ref/remastered/vgatext.spr` (1998-12-12) |
| `assets/vtitle.bmp` | 14,764 bytes | VGA title screen: 640 x 400, 4-bit, RLE4-compressed BMP with a 16-colour palette. | `ref/remastered/vtitle.bmp` (2001-08-26) |
| `assets/COPYING.digger-remastered` | 18,332 bytes | The GNU General Public License, version 2, as distributed with Digger Remastered. | `ref/remastered/COPYING` (byte-identical) |

SHA-256 of the bundled assets:

```
7dd8c4785a3b28bb054d6b4a6bf39015797d7d6d7f31881b007a66f908271430  assets/vga.spr
12260e1319d9a79fd3537b7d684c480b17d7a51309068a22d8820c7ae1016db4  assets/vgatext.spr
b928dc598cd0d2a18a0dddc23e7c651cbabacd97182e4f60ffa2829ee962f5e4  assets/vtitle.bmp
```

The three asset files are **byte-identical** to the copies in
`ref/remastered/`. (`ref/windows/vtitle.bmp` is a different file with a
different hash; the bundled title screen follows the Remastered tree.)
`ref/remastered/digsrc.txt` describes the formats:

> CGA.SPR      - CGA sprite data } These are all in CHARDES format. CHARDES is
> CGATEXT.SPR  - CGA font data   } available from my personal website [*]. The
> VGA.SPR      - VGA sprite data } format is just 1 byte per pixel, 32x32 format
> VGATEXT.SPR  - VGA font data   } data (no headers).

The bundled VGA files are `VGA.SPR` and `VGATEXT.SPR`; the CGA equivalents are
not bundled.

and:

> `VTITLE.BMP` - RLE encoded bitmap of the VGA title screen for linking into the
> Windows version

(`digsrc.txt:235-238,177-178`).

## Original Digger (1983)

The original game, source, binaries, and artwork are by Windmill Software.
`ref/remastered/digsrc.txt:44-53` states:

> Windmill Software holds the copyright to the original Digger source code and
> binaries, and have exclusive rights to sell, license and produce the game in
> its original form.
>
> Rob Sleath has the right to use the Digger source code for development of
> other projects. Digger Remastered is such a project. It was produced without
> reference to the original Digger source by Windmill software, and without help
> from the original developers of Digger in any capacity other than historical
> advice. With the permission of Rob Sleath, Digger Remastered is Copyright (c)
> Andrew Jenner 1998-2004.

The same text appears in `ref/windows/copyright.txt`. The original development
tree is preserved under `ref/original/` (`Source/`, `Work/`, `Sprites/`,
`C Compiler/`, `Disk Create/`); it is included as historical reference only.

## Digger Remastered is GPL-2.0-or-later

`ref/remastered/digsrc.txt:55-58` says:

> This program is free software; you can redistribute it and/or
> modify it under the terms of the GNU General Public License
> as published by the Free Software Foundation; either version 2
> of the License, or (at your option) any later version.

The full license text is reproduced in
`assets/COPYING.digger-remastered` (identical to `ref/remastered/COPYING`).
The "or any later version" wording is why this project uses
**GPL-2.0-or-later** for its own code, matching `Cargo.toml`:

```toml
license = "GPL-2.0-or-later"
```

The Rust sources are a new implementation, not a copy of the C sources, but
they are a port derived from Digger Remastered and are distributed under the
same GPL-2.0-or-later terms.

## The restrictive wording in `digsrc.txt`

The same file also contains a separate, more restrictive statement at
`digsrc.txt:21-25`:

> Note that Windmill Software owns copyright on this source code, so it is
> subject to some licensing restrictions. You cannot make any money from any
> versions of Digger you create using this code unless you have permission to
> do so from Windmill Software. All rights to any software created with this
> code must be relinquished to Windmill Software upon request.

This wording and the GPL grant coexist in the Digger Remastered release. The
GPL text grants redistribution and modification rights, while the quoted
paragraph says money-making requires Windmill Software's permission and that
rights must be relinquished on request. The two statements are in tension.
This project does **not** claim that one clause overrides the other, and does
not offer a legal interpretation. The ambiguity is recorded here so that it is
not lost.

## This project's position

* **Code:** GPL-2.0-or-later, consistent with the Digger Remastered release and
  `Cargo.toml`. If you distribute modified binaries, provide the corresponding
  source under the same license.
* **Assets:** `vga.spr`, `vgatext.spr`, and `vtitle.bmp` are included for
  **preservation and remake purposes**, under the terms of the Digger
  Remastered release. They remain the copyright of their respective owners
  (Windmill Software for the original Digger artwork; Andrew Jenner / the
  Digger Remastered authors for the Remastered versions). They are not
  relicensed by this project.
* **Intent:** the project is a non-commercial, archival/remake effort. It does
  not sell the game or the assets and does not claim ownership of them. Anyone
  intending to distribute the assets or a build commercially should seek
  permission from the rights holders (Windmill Software and/or the Digger
  Remastered authors) rather than relying on this note.
* **No warranty / no legal advice:** this document quotes the files and states
  the project's intent; it is not a legal opinion. If you need a definitive
  answer, consult the rights holders and a lawyer.

## Suggested attribution text

For an About screen, README, or distribution notice:

> Digger was created by Windmill Software in 1983. This remake is based on
> Digger Remastered by Andrew Jenner (1998-2004), the VGA sources and assets of
> which are distributed under the GNU General Public License, version 2 or
> later. Digger, its artwork, and the Digger Remastered assets remain the
> copyright of their respective owners and are included here for preservation
> and non-commercial remake purposes. This project's source code is licensed
> under GPL-2.0-or-later; see `assets/COPYING.digger-remastered`.

## See also

* `ref/remastered/digsrc.txt` - source layout, copyright and licensing text,
  asset format descriptions.
* `ref/remastered/COPYING` and `assets/COPYING.digger-remastered` - GPL-2.0.
* `ref/windows/copyright.txt` - the same Windmill/Andrew Jenner copyright
  statement.
* `docs/PORTING_NOTES.md` - technical provenance and the reference-tree map.
