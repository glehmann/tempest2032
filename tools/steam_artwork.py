#!/usr/bin/env python3
# /// script
# dependencies = ["cairosvg>=2.7", "pillow>=11"]
# ///
"""Regenerate embedded Steam artwork: uv run tools/steam_artwork.py."""

import argparse
from io import BytesIO
from pathlib import Path
import shutil
import xml.etree.ElementTree as ET

import cairosvg
from PIL import Image, ImageOps

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "steam"
NS = "http://www.w3.org/2000/svg"
ET.register_namespace("", NS)


def artwork(name, width, height, placements, background=True, caption=None):
    svg = ET.Element(f"{{{NS}}}svg", {"viewBox": f"0 0 {width} {height}"})
    if background:
        ET.SubElement(svg, f"{{{NS}}}rect", {
            "width": str(width), "height": str(height), "fill": "#070b1d",
        })
    for source, x, y, w, h in placements:
        logo = ET.parse(ROOT / "logos" / source).getroot()
        logo.attrib.update(x=str(x), y=str(y), width=str(w), height=str(h))
        svg.append(logo)
    if caption:
        x, y, size, anchor = caption
        defs = ET.SubElement(svg, f"{{{NS}}}defs")
        gradient = ET.SubElement(defs, f"{{{NS}}}linearGradient", {"id": "caption-spectrum"})
        for offset, color in [(0, "#ff55d8"), (.28, "#a978ff"), (.55, "#4ae5ff"),
                              (.78, "#9aff8a"), (1, "#ffe66a")]:
            ET.SubElement(gradient, f"{{{NS}}}stop", {"offset": str(offset), "stop-color": color})
        for text, baseline, font_size, color, spacing in [
            ("TEMPEST", y, size, "#f8f4ff", 0),
            ("2032", y + size * 1.15, size * .9, "url(#caption-spectrum)", 8),
        ]:
            ET.SubElement(svg, f"{{{NS}}}text", {
                "x": str(x), "y": str(baseline), "font-size": str(font_size),
                "font-family": "DejaVu Sans,Arial,sans-serif", "font-weight": "bold",
                "text-anchor": anchor, "fill": color, "letter-spacing": str(spacing),
            }).text = text
    png = cairosvg.svg2png(bytestring=ET.tostring(svg), output_width=width * 2,
                         output_height=height * 2)
    image = Image.open(BytesIO(png)).resize((width, height), Image.Resampling.LANCZOS)
    image.save(OUTPUT / name, optimize=True)
    print(f"{name}: {width}x{height}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hero-source", type=Path, help="Replace the saved gameplay screenshot")
    args = parser.parse_args()
    OUTPUT.mkdir(exist_ok=True)
    gameplay = OUTPUT / "gameplay.png"
    if args.hero_source and args.hero_source.resolve() != gameplay.resolve():
        shutil.copyfile(args.hero_source, gameplay)
    spectrum = "05-spectrum-portal.svg"
    artwork("icon.png", 256, 256, [("favicon.svg", 0, 0, 256, 256)], background=False)
    artwork("logo.png", 320, 320, [(spectrum, 0, 0, 320, 320)], background=False)
    artwork("landscape.png", 920, 430, [(spectrum, 40, 55, 320, 320)],
            caption=(400, 200, 64, "start"))
    artwork("portrait.png", 600, 900, [(spectrum, 70, 90, 460, 460)],
            caption=(300, 680, 64, "middle"))
    with Image.open(gameplay) as screenshot:
        # Big Picture crops the hero vertically and overlays the logo at bottom left.
        scene = ImageOps.contain(screenshot.convert("RGB"), (1680, 440),
                                 method=Image.Resampling.LANCZOS)
        hero = Image.new("RGB", (1920, 620))
        hero.paste(scene, ((hero.width - scene.width) // 2,
                           (hero.height - scene.height) // 2))
        hero.save(OUTPUT / "hero.png", optimize=True)
    print("hero.png: 1920x620 gameplay screenshot")
