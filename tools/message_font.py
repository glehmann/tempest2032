"""Extract scalable afont contours from the authorized original source tree."""

import re
import struct
import sys
from collections import Counter
from pathlib import Path


def contours(mask):
    edges = {}
    for y, row in enumerate(mask):
        for x, inside in enumerate(row):
            if not inside:
                continue
            for a, b, dx, dy in [
                ((x, y), (x + 1, y), 0, -1),
                ((x + 1, y), (x + 1, y + 1), 1, 0),
                ((x + 1, y + 1), (x, y + 1), 0, 1),
                ((x, y + 1), (x, y), -1, 0),
            ]:
                nx, ny = x + dx, y + dy
                if not (0 <= ny < len(mask) and 0 <= nx < len(row) and mask[ny][nx]):
                    edges.setdefault(a, []).append(b)
    result = []
    while edges:
        start = next(iter(edges))
        points = [start]
        point = start
        while True:
            target = edges[point].pop()
            if not edges[point]:
                del edges[point]
            point = target
            if point == start:
                break
            points.append(point)
        simplified = []
        for i, point in enumerate(points):
            before, after = points[i - 1], points[(i + 1) % len(points)]
            if (point[0] - before[0]) * (after[1] - point[1]) != (point[1] - before[1]) * (after[0] - point[0]):
                simplified.append(point)
        if len(simplified) >= 3:
            result.append(simplified)
    return result


def main():
    source, output = map(Path, sys.argv[1:])
    if output.name == "hud.bin":
        pixels = (source / "images/beasty4.cry").read_bytes()
        binary = bytearray(b"T32H")
        # yak.s ashowscore/setlives use these regions, not the announcement font.
        for ox, oy, width, height in [
            *[(148 + digit * 16, 89, 16, 19) for digit in range(10)],
            (148, 111, 26, 17),
        ]:
            crop = b"".join(
                pixels[((oy + y) * 320 + ox) * 2:((oy + y) * 320 + ox + width) * 2]
                for y in range(height)
            )
            mask = [[crop[(y * width + x) * 2 + 1] > 80 for x in range(width)] for y in range(height)]
            paths = contours(mask)
            assert paths
            binary.extend(struct.pack(">HHH", width, height, len(paths)))
            for points in paths:
                binary.extend(struct.pack(">H", len(points)))
                for x, y in points:
                    binary.extend(struct.pack(">hh", x, y))
            binary.extend(crop)
        output.write_bytes(binary)
        print(f"{output}: {len(binary)} bytes, 10 vector digits and the original life icon")
        return
    if output.name == "game-over.cry":
        pixels = (source / "images/beasty4.cry").read_bytes()
        output.write_bytes(b"".join(
            pixels[((21 + y) * 320 + 148) * 2:((21 + y) * 320 + 293) * 2]
            for y in range(68)
        ))
        print(f"{output}: original 145x68 Game Over CRY bitmap")
        return
    if output.name == "wordart.bin":
        binary = bytearray(b"T32C")
        for key, image, ox, oy, width, height in [
            (0,"beasty3.cry",137,110,178,42),
            (1,"beasty7.cry",4,84,197,65),
            (2,"beasty4.cry",148,21,145,68),
            (3,"beasty4.cry",148,0,50,21),
            (4,"beasty5.cry",0,110,45,44),
            (5,"beasty5.cry",46,110,44,44),
            (6,"beasty5.cry",90,110,45,44),
            (7,"beasty4.cry",198,0,15,21),
        ]:
            pixels = (source / "images" / image).read_bytes()
            mask = [[pixels[((oy+y)*320+ox+x)*2+1] > 80 for x in range(width)] for y in range(height)]
            paths = contours(mask)
            assert paths
            binary.extend(struct.pack(">BHH",key,width,height))
            for y in range(height):
                row = [pixels[((oy+y)*320+ox+x)*2] for x in range(width) if mask[y][x]]
                hue = Counter(row).most_common(1)[0][0] if row else 0
                binary.extend(struct.pack(">H",(hue << 8) | 240))
            binary.extend(struct.pack(">H",len(paths)))
            for points in paths:
                binary.extend(struct.pack(">H",len(points)))
                for x,y in points:
                    binary.extend(struct.pack(">hh",x,y))
            if key in (0, 1):
                for y in range(height):
                    start = ((oy+y)*320+ox)*2
                    binary.extend(pixels[start:start+width*2])
        output.write_bytes(binary)
        print(f"{output}: {len(binary)} bytes, 8 vector word-art graphics")
        return
    table = (source / "afont.s").read_text().split("afont:", 1)[1]
    values = [int(v, 16) for v in re.findall(r"dc\.l\s+\$([0-9a-f]+)", table, re.I)]
    height, width = values[0] >> 16, values[0] & 65535
    assert (width, height) == (15, 18)
    pixels = (source / "images/beasty4.cry").read_bytes()
    binary = bytearray(b"T32S")
    total = 0
    for character in " !(),-.:0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ":
        offset = values[ord(character) - 32 + 1]
        ox, oy = offset & 65535, offset >> 16
        # The sheet's dark bevel/background is intensity 74, not transparent zero.
        mask = [[pixels[((oy + y) * 320 + ox + x) * 2 + 1] > 80 for x in range(width)] for y in range(height)]
        paths = contours(mask) if character != " " else []
        assert paths or character == " "
        binary.extend(struct.pack(">BH", ord(character), len(paths)))
        for points in paths:
            binary.extend(struct.pack(">H", len(points)))
            for x, y in points:
                binary.extend(struct.pack(">hh", x, y))
                total += 1
        for y in range(height):
            start = ((oy + y) * 320 + ox) * 2
            binary.extend(pixels[start:start + width * 2])
    output.write_bytes(binary)
    print(f"{output}: {len(binary)} bytes, {total} contour vertices")


if __name__ == "__main__":
    main()
