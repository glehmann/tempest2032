import json
from pathlib import Path
import plistlib
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import package


class Packages(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.output = Path(self.temp.name)
        self.binary = self.output / "game"
        self.binary.write_bytes(b"standalone game")

    def test_linux_archive_is_self_contained_and_executable(self):
        package.native("linux", "x86_64-unknown-linux-gnu", self.binary, self.output)
        with tarfile.open(next(self.output.glob("*.tar.gz"))) as archive:
            files = archive.getmembers()
            binary = next(file for file in files if file.name.endswith("/tempest2032"))
            self.assertEqual(binary.mode, 0o755)
            self.assertEqual(archive.extractfile(binary).read(), b"standalone game")
            self.assertTrue(any(file.name.endswith("/README.txt") for file in files))
            self.assertFalse(any("/assets/" in file.name for file in files))

    def test_mac_bundle_and_applications_link_are_inside_dmg_source(self):
        def command(args, **kwargs):
            if args[:2] != ["hdiutil", "create"]:
                return
            stage = Path(args[args.index("-srcfolder") + 1])
            contents = stage / "Tempest 2032.app/Contents"
            with (contents / "Info.plist").open("rb") as file:
                info = plistlib.load(file)
            self.assertEqual(info["CFBundleIdentifier"], package.APP_ID)
            self.assertEqual(info["CFBundleExecutable"], "tempest2032")
            self.assertTrue((contents / "Resources/icon.icns").is_file())
            self.assertEqual((contents / "MacOS/tempest2032").read_bytes(), b"standalone game")
            self.assertEqual((stage / "Applications").readlink(), Path("/Applications"))

        with patch("package.subprocess.run", side_effect=command) as run:
            package.native("macos", "aarch64-apple-darwin", self.binary, self.output)
        self.assertEqual([call.args[0][0] for call in run.call_args_list],
                         ["codesign", "codesign", "hdiutil", "hdiutil"])

    def test_flatpak_dependencies_have_checksums_and_source_is_pinned(self):
        revision = "a" * 40
        package.flatpak(self.output, revision)
        manifest = json.loads((self.output / f"{package.APP_ID}.json").read_text())
        self.assertEqual(manifest["modules"][0]["sources"][0]["commit"], revision)
        sources = json.loads((self.output / "cargo-sources.json").read_text())
        archives = [source for source in sources if source["type"] == "archive"]
        self.assertGreater(len(archives), 100)
        self.assertTrue(all(len(source["sha256"]) == 64 for source in archives))
        local = json.loads((self.output / "local.json").read_text())
        self.assertEqual(local["modules"][0]["sources"][0], {"type": "dir", "path": "source"})
        self.assertTrue((self.output / "source/assets/steam/icon.png").is_file())
        with tarfile.open(self.output / "flathub-submission.tar.gz") as archive:
            self.assertEqual(set(archive.getnames()), {f"{package.APP_ID}.json", "cargo-sources.json"})

    def test_windows_installer_is_checked_and_uninstalled(self):
        def command(args, **kwargs):
            if "/VERYSILENT" not in args or args[0].endswith("unins000.exe"):
                return
            installed = Path(next(arg[5:] for arg in args if arg.startswith("/DIR=")))
            installed.mkdir()
            (installed / "tempest2032.exe").write_bytes(self.binary.read_bytes())
            (installed / "unins000.exe").touch()

        with patch("package.shutil.which", return_value="ISCC"), \
                patch("package.subprocess.run", side_effect=command) as run:
            package.native("windows", "x86_64-pc-windows-msvc", self.binary, self.output)
        self.assertEqual(len(run.call_args_list), 3)
        self.assertTrue(run.call_args_list[-1].args[0][0].endswith("unins000.exe"))

    def test_flatpak_rejects_unpinned_revisions(self):
        with self.assertRaises(ValueError):
            package.flatpak(self.output, "main")


if __name__ == "__main__":
    unittest.main()
