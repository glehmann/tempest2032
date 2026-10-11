# Lettering preview lab

This example renders typography candidates without starting the game or changing
its fonts. It reuses the game's actual extrusion, bevels, colour bands, and fixed
studio shading. Candidate front shapes are SVG paths loaded from
`tools/lettering-preview.toml` at runtime.

## Comparison sheets

```sh
cargo run --example lettering_preview
cargo run --example lettering_preview -- --background web
```

The PNGs are written to `target/lettering-preview/comparison-black.png` and
`comparison-web.png`. Each sheet contains the current game font and A–D, with
straight and slanted poses side by side. All rows use the same visible cap height,
advance, palette, bevel, depth, lighting, and pose transforms.

| ID | Shape family | Weight |
| --- | --- | --- |
| current | Current game lettering | Baseline |
| A | Original-inspired rounded bowls | Medium |
| B | Same shapes as A | Light |
| C | More rounded bowls and terminals | Stronger medium |
| D | Same shapes as C | Light |

A has nominal three-unit strokes. C expands the original rounded outlines by
0.15 units, increasing nominal stroke width to approximately 3.3 units before
cap-height normalization. The light candidates move
outer boundaries inward and hole boundaries outward by 0.25 units, reducing
stroke width by approximately 0.5 units. Cap height is normalized afterward.
Both candidate families are thinner than the current sprite-derived outlines.

## Individual PNGs

```sh
cargo run --example lettering_preview -- --variant A --pose straight
cargo run --example lettering_preview -- --variant D --pose slanted --background web
cargo run --example lettering_preview -- --variant B --output target/lettering-preview/B.png
```

`--pose` accepts `both`, `straight`, or `slanted`. `--variant` accepts a candidate
ID from the file or `current`; omitting it renders the entire sheet. Use
`--width` and `--height` for a different output size. Individual rows default to
432 pixels high; both-pose sheets default to 3840 pixels wide.

The resizable viewer starts centered on the primary monitor. It fits within 80%
of that monitor's size, accounting for desktop scaling, and is capped at
1600 by 900 logical pixels. The whole sheet fits inside the window when resized.
If the desktop does not report a primary monitor, it uses the window's monitor.
PNG exports render separately at the requested pixel dimensions, including labels,
regardless of the window size or desktop scaling.

## Live iteration

```sh
cargo run --example lettering_preview -- --watch
```

Edit `tools/lettering-preview.toml` while the window is open. Changes reload after
250 ms and produce a fresh PNG once the new render has settled. No Rust or game
rebuild is needed. R reloads manually, S saves the current view, and Esc exits.
Invalid edits retain the last valid preview and report an error in the terminal.
Changing the number of rows or canvas dimensions requires restarting the tool.

The file controls the message, advance, cap height, slant, candidate family, inset,
and each glyph's front-face SVG path. The paths support absolute and relative
M, L, H, V, C, Q, and Z commands. Use a closed subpath for each outer boundary
and another for each letter opening. Cubic and quadratic curves are flattened
with a 0.012-unit tolerance before generating the same 3D shell as the game.
Inset accepts -0.25 through 0.75. Negative values thicken strokes; positive values
thin them. Letter openings shrink when strokes thicken and grow when they thin.

To mix designs, copy individual path entries between `[fonts.original]` and
`[fonts.rounded]`, or add a new family and variant. Palette and lighting stay
fixed so each comparison measures typography rather than material changes.

## Checks

```sh
cargo test --example lettering_preview
cargo clippy --example lettering_preview -- -D warnings
```
