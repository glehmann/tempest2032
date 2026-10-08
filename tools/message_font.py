"""Extract scalable afont contours from the authorized original source tree."""

import re
import struct
import sys
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
    table = (source / "afont.s").read_text().split("afont:", 1)[1]
    values = [int(v, 16) for v in re.findall(r"dc\.l\s+\$([0-9a-f]+)", table, re.I)]
    height, width = values[0] >> 16, values[0] & 65535
    assert (width, height) == (15, 18)
    pixels = (source / "images/beasty4.cry").read_bytes()
    binary = bytearray(b"T2KF")
    total = 0
    for character in " !(),-.:0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ":
        offset = values[ord(character) - 32 + 1]
        ox, oy = offset & 65535, offset >> 16
        mask = [[pixels[((oy + y) * 320 + ox + x) * 2 + 1] > 8 for x in range(width)] for y in range(height)]
        paths = contours(mask) if character != " " else []
        assert paths or character == " "
        binary.extend(struct.pack(">BH", ord(character), len(paths)))
        for points in paths:
            binary.extend(struct.pack(">H", len(points)))
            for x, y in points:
                binary.extend(struct.pack(">hh", x, y))
                total += 1
    output.write_bytes(binary)
    print(f"{output}: {len(binary)} bytes, {total} contour vertices")


if __name__ == "__main__":
    main()
