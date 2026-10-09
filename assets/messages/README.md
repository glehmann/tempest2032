# Message lettering

`afont.bin` contains scalable outlines derived from Joby's `afont` letters in
the original `beasty4.cry` sheet. `tools/message_font.py` extracts the glyph
boundaries and removes redundant contour points. The renderer rounds the
contour corners with quadratic curves and tessellates at the display scale.
Glyphs retain the original 15×18 proportions and 17-unit advance. Lowercase
characters use the original uppercase forms.

`wordart.bin` contains outlines of the original EXCELLENT, 2000, GAME OVER,
1UP, 250, 500, and 750 artwork. Source rectangles follow `any_pixex`,
`draw_oneup`, `gofeed`, and `px_bons` in `yak.s`. Its `T2KC` records include
the artwork's width, height, and one CRY shading color per row before the contour count. Generate it with
the same command, using `assets/messages/wordart.bin` as the output.
The 1UP badge combines the UP graphic and its separately stored numeral,
with the smaller numeral above the main lettering.
The EXCELLENT and 2000 records also store their original CRY pixels after the
contours. Their approach uses one opaque, fixed-size quad per foreground source
pixel rather than enlarged solid artwork or overlapping echoes.

The binary starts with `T2KS`, followed by glyph records: ASCII byte, contour
count (big-endian u16), then point count (u16) and pairs of signed i16 X/Y
coordinates. Each record then contains the original 15×18 big-endian CRY
shading samples. The smooth outline is clipped into a fixed 15×18 cell grid
at startup. Cell vertices sample these colors, preserving the white reflections,
dark bevels, and colored face at native resolution. Cells share edges while
held, then move apart during expansion without growing or fading. Layouts and
linear colors are cached; recursive interior subdivision is not used.
The reference source tree is not needed at runtime.

`game-over.cry` retains the full 145x68 CRY bitmap used by `gofeed`, from
`beasty4.cry` at source coordinates 148,21. Its highlights and shading are
preserved per pixel. Game Over renders this centered bitmap at `rex`'s depth 96,
pulsing its X/Y scale with phase increments 15 and 17. Each 60 Hz tick scales
and rotates the previous 384x240 image with `scar`'s scale 500 and cyclic angle
from -7 to 8, subtracts eight intensity units, then stamps the bitmap again.
The feedback is opaque CRY pixels rather than translucent vector copies.

Regenerate the Game Over bitmap with:

```sh
python3 tools/message_font.py /path/to/tempest2k/src assets/messages/game-over.cry
```

Regenerate with:

```sh
python tools/message_font.py /path/to/tempest2k/src assets/messages/afont.bin
```
