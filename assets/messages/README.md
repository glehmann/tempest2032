# Message lettering

The HUD uses hand-defined polygonal score glyphs in `src/messages/geometry.rs`.
Ordinary announcements use the original `afont` contours as shape references,
with pixel stair-steps simplified and corners reconstructed as circular arcs.
Their fronts, bevels, sides, and backs are
real 3D triangles. Continuous colour bands and fixed normal-based studio shading
replace the old bitmap-derived highlights. Only normal screen-resolution
anti-aliasing is applied; there is no edge-feathering or shading texture.
The score's extrusion is 3.2 logical units deep, offset 1.6 units right and
2.4 units down; other lettering retains the shallower extrusion.
PAUSED uses red fronts and gold bevels, with the original large `bfont`'s
27x31 proportions and 29-unit advance. Life icons reuse `claw_triangles(4)`.

`hud.bin` stores the ten original 16x19 score numerals and the 26x17 claw life
icon from `beasty4.cry`. Source rectangles match `ashowscore` and `setlives`
in `yak.s`. Its `T32H` header is followed by eleven records containing width,
height, contour count, contour points, and CRY shading samples, all big-endian
u16 values. It is retained as a reference asset for the original artwork;
the runtime HUD uses the new 3D geometry. Warp pyramids retain `makepyr`'s
three shaded triangles.

Regenerate the HUD artwork with:

```sh
python3 tools/message_font.py /path/to/tempest2000/src assets/messages/hud.bin
```

`afont.bin` contains scalable outlines derived from Joby's `afont` letters in
the original `beasty4.cry` sheet. `tools/message_font.py` extracts the glyph
boundaries and removes redundant contour points. At startup, the renderer
simplifies the pixel stair-steps with a 0.8-source-pixel line-fit tolerance,
then rounds retained corners with quadratic curves capped at 0.75 source
pixels. Long diagonals remain straight instead of following each pixel step.
The outlines are retained for reference and the legacy approach-effect renderer.
Glyphs retain the original 15×18 proportions and 17-unit advance. Lowercase
characters use the original uppercase forms.

`wordart.bin` contains outlines of the original EXCELLENT, 2000, GAME OVER,
1UP, 250, 500, and 750 artwork. Source rectangles follow `any_pixex`,
`draw_oneup`, `gofeed`, and `px_bons` in `yak.s`. Its `T32C` records include
the artwork's width, height, and one CRY shading color per row before the contour count. Generate it with
the same command, using `assets/messages/wordart.bin` as the output.
The 1UP badge combines the UP graphic and its separately stored numeral,
with the smaller numeral above the main lettering.
The EXCELLENT, 2000, UP, and separate 1 records also store their original CRY
pixels after the contours; the runtime skips every outline and row colour and
reads only these pixels. Their approach uses one opaque, fixed-size quad per foreground
source pixel rather than enlarged solid artwork or overlapping echoes. The
2000 renderer treats CRY intensities below 32 as transparent to trim its darkest
cells. EXCELLENT retains every nonzero source pixel, including dark shading. The
1UP uses the original UP pixels and half-sized 1, with CRY intensities above 80
to exclude the sheet's dark background. Its cells separate with the same approach
as 2000 and EXCELLENT. The 250/500/750 outlines remain reference assets; their
runtime labels use the cell-based announcement font.

The binary starts with `T32S`, followed by glyph records: ASCII byte, contour
count (big-endian u16), then point count (u16) and pairs of signed i16 X/Y
coordinates. Each record then contains the original 15×18 big-endian CRY
shading samples. The legacy decoder can reconstruct the original sampled
lettering for reference; ordinary announcements now use the new geometric
glyphs and procedural shading. Their cached fragment surfaces retain their size
when they separate during the original stretch animation. The reference source
tree is not needed at runtime.

`game-over.cry` retains the full 145x68 CRY bitmap used by `gofeed`, from
`beasty4.cry` at source coordinates 148,21. Its highlights and shading are
preserved per pixel. Game Over renders this centered bitmap at `rex`'s depth 96,
pulsing its X/Y scale with phase increments 15 and 17. The original advances
`gofeed` once per completed GPU frame through `gogame`/`thang`/`db`, rather than
through the VBlank gameplay timer. The port paces the entire effect at 15 Hz,
holding each feedback frame for four 60 Hz simulation ticks. The previous
bitmap foreground and feedback are restored without a sharp 3D text overlay. Each feedback step
scales and rotates the previous 384x240 image with `scar`'s scale 500 and cyclic angle
from -7 to 8, subtracts eight intensity units, then stamps the bitmap again.
The feedback is opaque CRY pixels rather than translucent vector copies.

Regenerate the Game Over bitmap with:

```sh
python3 tools/message_font.py /path/to/tempest2000/src assets/messages/game-over.cry
```

Regenerate with:

```sh
python tools/message_font.py /path/to/tempest2000/src assets/messages/afont.bin
```

`warp-victory.cry` is `vicbon`'s full 178x68 source rectangle from
`beasty3.cry` at 137,110. It includes both EXCELLENT and 20000 BONUS.
`warp-five.cry` stores the 238x18 original afont message merged into the final
completion fade. During the completion screen, that message uses the native
3D lettering above the bitmap feedback.

```sh
python3 tools/message_font.py /path/to/tempest2000/src assets/messages/warp-victory.cry
python3 tools/message_font.py /path/to/tempest2000/src assets/messages/warp-five.cry
```
