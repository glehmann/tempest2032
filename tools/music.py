"""Extract the seven original MOD tracks from the source tree or Jaguar ROM."""

import argparse
import hashlib
import struct
from pathlib import Path


TRACKS = [
    ("tune13", 0x0D6900),
    ("tune7", 0x0EF52C),
    ("tune1", 0x10E204),
    ("tune3", 0x13257C),
    ("rave4", 0x157322),
    ("tune5", 0x170572),
    ("tune12", 0x187250),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="original source tree or .j64 cartridge")
    parser.add_argument("output", type=Path, help="output directory")
    args = parser.parse_args()
    rom = args.source.read_bytes() if args.source.is_file() else None
    if rom is not None and hashlib.md5(rom).hexdigest() != "602bc9953d3737b1ba52b2a0d9932f7c":
        raise ValueError("expected the original 2 MiB Tempest 2000 cartridge")
    args.output.mkdir(parents=True, exist_ok=True)
    for name, offset in TRACKS:
        data = rom[offset:] if rom else (args.source / f"src/sounds/{name}.mod").read_bytes()
        if data[1080:1084] != b"M.K.":
            raise ValueError(f"invalid MOD header for {name}")
        length = 1084 + (max(data[952:1080]) + 1) * 1024
        length += sum(struct.unpack_from(">H", data, 42 + i * 30)[0] * 2 for i in range(31))
        (args.output / f"{name}.mod").write_bytes(data[:length])
        print(f"{name}: {length} bytes")


if __name__ == "__main__":
    main()
