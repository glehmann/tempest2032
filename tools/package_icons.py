#!/usr/bin/env python3
# /// script
# dependencies = ["cairosvg>=2.7", "pillow>=11"]
# ///
"""Regenerate Windows and macOS application icons from the embedded favicon."""

from io import BytesIO
from pathlib import Path

import cairosvg
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
image = Image.open(BytesIO(cairosvg.svg2png(
    url=str(ROOT / "logos/favicon.svg"), output_width=1024, output_height=1024,
)))
for platform, name, format in [("windows", "icon.ico", "ICO"), ("macos", "icon.icns", "ICNS")]:
    directory = ROOT / "packaging" / platform
    directory.mkdir(parents=True, exist_ok=True)
    image.save(directory / name, format=format)
