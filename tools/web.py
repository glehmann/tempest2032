#!/usr/bin/env python3
"""Build the standalone browser game or serve its generated files locally."""

import argparse
import functools
import http.server
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "target/web"


def build(profile):
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
    version = next(p["version"] for p in lock["package"] if p["name"] == "wasm-bindgen")
    install = f"cargo install wasm-bindgen-cli --version {version} --locked"
    if not shutil.which("wasm-bindgen"):
        raise SystemExit(f"Install the bindings tool first: {install}")
    actual = subprocess.check_output(["wasm-bindgen", "--version"], text=True).strip()
    if actual != f"wasm-bindgen {version}":
        raise SystemExit(f"Install matching bindings: {install}")
    subprocess.run(
        ["cargo", "build", "--locked", "--target", "wasm32-unknown-unknown", "--profile", profile],
        cwd=ROOT, check=True,
    )
    OUTPUT.mkdir(parents=True, exist_ok=True)
    directory = "debug" if profile == "dev" else profile
    subprocess.run([
        "wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(OUTPUT),
        *(["--remove-name-section"] if profile == "web-release" else []),
        "--out-name", "tempest2032",
        str(ROOT / f"target/wasm32-unknown-unknown/{directory}/tempest2032.wasm"),
    ], check=True)
    for path in (ROOT / "web").iterdir():
        if path.is_file():
            shutil.copy2(path, OUTPUT / path.name)
    for name in ["LICENSE", "NOTICE"]:
        shutil.copy2(ROOT / name, OUTPUT / name)
    shutil.copy2(ROOT / "assets/steam/icon.png", OUTPUT / "favicon.png")
    shutil.copy2(ROOT / "assets/preview.png", OUTPUT / "preview.png")
    print(f"Browser build written to {OUTPUT}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["build", "serve"])
    parser.add_argument("--profile", default="web-release", choices=["dev", "web-release"])
    parser.add_argument("--port", type=int, default=8000)
    args = parser.parse_args()
    if args.command == "build":
        build(args.profile)
    else:
        if not (OUTPUT / "tempest2032_bg.wasm").exists():
            parser.error("Build the browser game first with make web")
        handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(OUTPUT))
        with http.server.ThreadingHTTPServer(("127.0.0.1", args.port), handler) as server:
            print(f"Play at http://127.0.0.1:{args.port}", flush=True)
            try:
                server.serve_forever()
            except KeyboardInterrupt:
                pass


if __name__ == "__main__":
    main()
