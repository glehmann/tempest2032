import json
from pathlib import Path
import re
import shutil
import struct
import tempfile
import unittest

import web


class BrowserBuild(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.output = Path(self.temp.name)
        self.templates()

    def templates(self):
        for path in (web.ROOT / "web").iterdir():
            shutil.copy2(path, self.output / path.name)
        (self.output / "tempest2032.js").write_text("new URL('tempest2032_bg.wasm', import.meta.url)")
        (self.output / "tempest2032_bg.wasm").write_bytes(b"embedded game")

    def test_install_manifest_has_fullscreen_launch_and_matching_png_icons(self):
        manifest = json.loads((self.output / "manifest.webmanifest").read_text())
        self.assertEqual(manifest["display"], "fullscreen")
        for name in ["id", "start_url", "scope"]:
            self.assertEqual(manifest[name], "./")
        self.assertEqual({icon["sizes"] for icon in manifest["icons"]}, {"192x192", "512x512"})
        for icon in manifest["icons"]:
            data = (self.output / icon["src"]).read_bytes()
            self.assertEqual(data[:8], b"\x89PNG\r\n\x1a\n")
            width, height = struct.unpack(">II", data[16:24])
            self.assertEqual(icon["sizes"], f"{width}x{height}")

    def test_precache_uses_the_same_urls_as_the_page_loader_and_wasm_bindings(self):
        version = web.finalize(self.output)
        worker = (self.output / "sw.js").read_text()
        files = json.loads(re.search(r"const FILES = (.*);", worker)[1])
        expected = {
            f"./{path.name}" + (f"?v={version}" if path.name in web.VERSIONED else "")
            for path in self.output.iterdir() if path.name != "sw.js"
        }
        self.assertEqual(set(files), expected)
        self.assertNotIn("__VERSION__", worker)
        self.assertNotIn("__PRECACHE__", worker)
        for file in web.VERSIONED:
            self.assertIn(f"./{file}?v={version}", files)
        self.assertIn(f"browser.js?v={version}", (self.output / "index.html").read_text())
        self.assertIn(f"tempest2032.js?v={version}", (self.output / "browser.js").read_text())
        self.assertIn(f"tempest2032_bg.wasm?v={version}", (self.output / "tempest2032.js").read_text())

    def test_rebuilding_replaces_the_generated_worker_without_changing_the_version(self):
        first = web.finalize(self.output)
        self.templates()
        self.assertEqual(web.finalize(self.output), first)

    def test_asset_changes_get_a_new_cache_and_versioned_loader_urls(self):
        first = web.finalize(self.output)
        self.templates()
        (self.output / "touch.js").write_text("updated touch controls")
        second = web.finalize(self.output)
        self.assertNotEqual(first, second)
        self.assertIn(second, (self.output / "sw.js").read_text())
        self.assertIn(second, (self.output / "index.html").read_text())


if __name__ == "__main__":
    unittest.main()
