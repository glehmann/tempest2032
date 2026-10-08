# Message lettering

`afont.bin` contains scalable outlines derived from Joby's `afont` letters in
the original `beasty4.cry` sheet. `tools/message_font.py` extracts the glyph
boundaries and removes redundant contour points. The renderer rounds the
contour corners with quadratic curves and tessellates at the display scale.
Glyphs retain the original 15×18 proportions and 17-unit advance. Lowercase
characters use the original uppercase forms.

`wordart.bin` contains outlines of the original EXCELLENT, 2000, GAME OVER,
1UP, 250, 500, and 750 artwork. Source rectangles follow `any_pixex`,
`draw_oneup`, `gofeed`, and `px_bons` in `yak.s`. Its `T2KA` records include
the artwork's width, height, and one CRY shading color per row before the contour count. Generate it with
the same command, using `assets/messages/wordart.bin` as the output.

The binary starts with `T2KF`, followed by glyph records: ASCII byte, contour
count (big-endian u16), then point count (u16) and pairs of signed i16 X/Y
coordinates. Only vector outlines are embedded; source bitmaps are not needed
at runtime. Original chroma shading is reproduced with vertex gradients.

Regenerate with:

```sh
python tools/message_font.py /path/to/tempest2k/src assets/messages/afont.bin
```
