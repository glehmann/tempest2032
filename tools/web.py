#!/usr/bin/env python3
"""Build the standalone browser game or serve its generated files locally."""

import argparse
import functools
import hashlib
import http.server
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "target/web"
# The page and the game call into each other, so a cached script from an older
# build breaks the new one. These reference each other and are left out of the hash.
PAGE = {"index.html", "browser.js"}


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
    version = tag(OUTPUT)
    replace(OUTPUT / "index.html", 'src="browser.js"', f'src="browser.js?v={version}"')
    replace(OUTPUT / "browser.js", "'./tempest2032.js'", f"'./tempest2032.js?v={version}'")
    print(f"Browser build written to {OUTPUT} (version {version})")


def tag(directory):
    """Summarizes everything the build produced, so the page and the game stay one version."""
    digest = hashlib.sha256()
    for path in sorted(directory.iterdir()):
        if path.is_file() and path.name not in PAGE:
            digest.update(path.name.encode())
            digest.update(path.read_bytes())
    # The published page and loader carry the tag, so hash the templates they are cut from.
    for name in sorted(PAGE):
        digest.update(name.encode())
        digest.update((ROOT / "web" / name).read_bytes())
    return digest.hexdigest()[:8]


def replace(path, old, new):
    text = path.read_text()
    if old not in text:
        raise SystemExit(f"{path.name} no longer contains {old}")
    path.write_text(text.replace(old, new))


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
