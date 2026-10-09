"""Pack original signed 8-bit PCM effects from the source tree or Jaguar ROM."""

import argparse
import hashlib
import struct
from pathlib import Path


def extract(source):
    rom = source.read_bytes() if source.is_file() else None
    if rom is not None:
        if hashlib.md5(rom).hexdigest() != "602bc9953d3737b1ba52b2a0d9932f7c":
            raise ValueError("expected the original 2 MiB Tempest 2000 cartridge")
        table = rom[0x1AC800:0x1ACD00]
    else:
        table = (source / "src/incbin/sound_samples_table.bin").read_bytes()
    output = bytearray(b"T2KS" + struct.pack("<I", 30))
    for index in range(30):
        name = table[index * 40:index * 40 + 20].decode("ascii").strip()
        period, start, length, repeat, repeat_length = struct.unpack_from(
            ">HIIII", table, index * 40 + 22
        )
        length >>= 8
        repeat_length >>= 8
        if rom is None:
            data = (source / f"src/sounds/samples/{index + 1:02}").read_bytes()[:length]
        elif start:
            # The normal-shot table length overruns its sample into the next one.
            next_start = struct.unpack_from(">I", table, (index + 1) * 40 + 24)[0]
            length = min(length, next_start - start - 4) if next_start > start else length
            data = rom[start - 0x800000:start - 0x800000 + length]
        else:
            data = b""
        loop_start = repeat - start if repeat_length and start else 0
        if repeat_length and loop_start + repeat_length > len(data):
            raise ValueError(f"invalid loop in sample {index}: {name}")
        output.extend(struct.pack("<HIII", period, len(data), loop_start, repeat_length))
        output.extend(data)
        print(f"{index:2}: {name:20} {len(data):6} bytes, period {period}")
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="original source tree or .j64 cartridge")
    parser.add_argument("output", type=Path, help="output sound pack")
    args = parser.parse_args()
    args.output.write_bytes(extract(args.source))


if __name__ == "__main__":
    main()
