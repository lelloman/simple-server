"""Exercise artifact installation without a release or network access."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
ENGINE = Path(os.environ["SIMPLE_SERVER_ENGINE_DIR"]) / "libsimple_server_engine.so"
with ENGINE.open("rb") as engine:
    MACHINE = int.from_bytes(engine.read(20)[18:20], "little")
TARGET = "x86_64-unknown-linux-gnu" if MACHINE == 62 else "aarch64-unknown-linux-gnu"


class ArtifactInstallation(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workspace = tempfile.TemporaryDirectory(prefix="engine-installer-")
        cls.root = Path(cls.workspace.name)
        cls.digest = hashlib.sha256(ENGINE.read_bytes()).hexdigest()
        shutil.copyfile(ROOT / "crates/engine-sys/build.rs", cls.root / "build.rs")
        (cls.root / "artifacts.sha256").write_text(f"{cls.digest} {TARGET}\n")
        env = dict(os.environ, CARGO_PKG_VERSION="0.2.0")
        subprocess.run(["rustc", "--edition=2024", str(cls.root / "build.rs"), "-o", str(cls.root / "install")], env=env, check=True)
        binary = cls.root / "bin"
        binary.mkdir()
        curl = binary / "curl"
        curl.write_text("#!/usr/bin/env python3\nimport os, shutil, sys\n"
                        "assert sys.argv[sys.argv.index('--proto') + 1] == '=https'\n"
                        "shutil.copyfile(os.environ['TEST_ENGINE_SOURCE'], sys.argv[sys.argv.index('--output') + 1])\n")
        curl.chmod(0o755)

    @classmethod
    def tearDownClass(cls):
        cls.workspace.cleanup()

    def setUp(self):
        self.case = tempfile.TemporaryDirectory(dir=self.root)
        self.addCleanup(self.case.cleanup)
        self.directory = Path(self.case.name)
        out = self.directory / "out"
        out.mkdir()
        self.env = dict(os.environ, TARGET=TARGET, OUT_DIR=str(out),
                        SIMPLE_SERVER_ENGINE_CACHE=str(self.directory / "cache"),
                        SIMPLE_SERVER_ENGINE_OFFLINE="false", CARGO_NET_OFFLINE="false",
                        TEST_ENGINE_SOURCE=str(ENGINE), PATH=f"{self.root / 'bin'}:{os.environ['PATH']}")
        self.env.pop("SIMPLE_SERVER_ENGINE_DIR", None)

    def run_installer(self):
        return subprocess.run([str(self.root / "install")], env=self.env,
                              capture_output=True, text=True, check=False)

    def test_download_is_verified_and_cached_for_offline_reuse(self):
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stderr)
        installed = Path(self.env["OUT_DIR"]) / "libsimple_server_engine.so.1"
        self.assertEqual(hashlib.sha256(installed.read_bytes()).hexdigest(), self.digest)
        self.env["SIMPLE_SERVER_ENGINE_OFFLINE"] = "true"
        self.env["TEST_ENGINE_SOURCE"] = "/must-not-download"
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_offline_cache_miss_is_an_error(self):
        self.env["SIMPLE_SERVER_ENGINE_OFFLINE"] = "true"
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("offline mode forbids downloading", result.stderr)

    def test_corrupt_download_is_rejected_before_linking(self):
        wrong = self.directory / "corrupt.so"
        wrong.write_bytes(b"not an engine")
        self.env["TEST_ENGINE_SOURCE"] = str(wrong)
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertFalse((Path(self.env["OUT_DIR"]) / "libsimple_server_engine.so").exists())
        self.assertFalse(list((self.directory / "cache").rglob("download-*.so")))

    def test_corrupt_cache_is_rejected(self):
        cache = self.directory / "cache" / "0.2.0" / TARGET / self.digest
        cache.mkdir(parents=True)
        (cache / "libsimple_server_engine.so").write_bytes(b"corruption")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cached engine checksum mismatch", result.stderr)

    def test_override_still_checks_target_architecture(self):
        self.env["SIMPLE_SERVER_ENGINE_DIR"] = str(ENGINE.parent)
        self.env["TARGET"] = "aarch64-unknown-linux-gnu" if MACHINE == 62 else "x86_64-unknown-linux-gnu"
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("engine architecture does not match", result.stderr)


if __name__ == "__main__":
    unittest.main()
