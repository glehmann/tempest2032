#!/usr/bin/env python3
# /// script
# dependencies = ["cairosvg>=2.7"]
# ///
"""Regenerate the browser installation icons from the shared favicon."""

from pathlib import Path

import cairosvg

ROOT = Path(__file__).resolve().parents[1]
for size in [192, 512]:
    cairosvg.svg2png(
        url=str(ROOT / "logos/favicon.svg"),
        output_width=size,
        output_height=size,
        write_to=str(ROOT / f"web/icon-{size}.png"),
    )
