"""Per-build pressure profiles use native BitBake configuration without user-file edits."""

import importlib.util
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from .support import BRIDGE


class BuildPerformanceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("bridge_performance", BRIDGE)
        cls.bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.bridge)

    def params(self, value, version="2.18.0"):
        with patch.dict(os.environ, {"YOCTUI_BUILD_PERFORMANCE": value}):
            return self.bridge.performance_config_params(
                SimpleNamespace(__version__=version),
                SimpleNamespace(
                    TinfoilConfigParameters=lambda **kwargs: SimpleNamespace(
                        options=SimpleNamespace(**kwargs), **kwargs
                    )
                ),
            )

    def test_default_does_not_create_profile_or_change_configuration(self):
        with patch("os.mkdir", side_effect=AssertionError("unexpected file write")):
            self.assertIsNone(self.params(""))

    def test_invalid_profiles_and_unsupported_bitbake_fail_closed(self):
        for value in [
            "[]",
            "{}",
            '{"cpu":true,"io":1,"memory":1}',
            '{"cpu":0,"io":1,"memory":1}',
            '{"cpu":1,"io":1,"memory":1000001}',
        ]:
            with self.subTest(value=value), self.assertRaises(RuntimeError):
                self.params(value)
        with self.assertRaisesRegex(RuntimeError, "2.2"):
            self.params('{"cpu":15000,"io":15000,"memory":15000}', "2.0.0")
        with self.assertRaisesRegex(RuntimeError, "verify"):
            self.params('{"cpu":15000,"io":15000,"memory":15000}', "unknown")

    def test_native_postread_file_is_stable_and_reused_without_touching_mtime(self):
        # Actual /proc PSI is read; no BitBake server is started by this boundary test.
        if not Path("/proc/pressure/memory").exists():
            self.skipTest("Linux PSI unavailable")
        with (
            tempfile.TemporaryDirectory() as directory,
            patch("tempfile.gettempdir", return_value=directory),
        ):
            value = json.dumps({"cpu": 15000, "io": 20000, "memory": 25000})
            params = self.params(value)
            self.assertEqual(params.options.postfile, params.postfile)
            profile = Path(params.postfile[0])
            first = profile.stat().st_mtime_ns
            self.assertEqual(
                profile.read_text(),
                '# Yoctui reviewed per-build optimization profile\nBB_PRESSURE_MAX_CPU = "15000"\nBB_PRESSURE_MAX_IO = "20000"\nBB_PRESSURE_MAX_MEMORY = "25000"\n',
            )
            self.assertEqual(self.params(value).postfile, params.postfile)
            self.assertEqual(profile.stat().st_mtime_ns, first)
            self.assertEqual(profile.parent.stat().st_mode & 0o777, 0o700)
            self.assertTrue(params.config_only)
            self.assertNotEqual(
                self.params(
                    json.dumps({"cpu": 15001, "io": 20000, "memory": 25000})
                ).postfile,
                params.postfile,
            )

    def test_alternative_connector_cannot_silently_ignore_the_profile(self):
        adapter = self.bridge.BitBakeAdapter(
            "2.18.0", SimpleNamespace(server=SimpleNamespace(connect=lambda: object()))
        )
        with patch.dict(
            os.environ,
            {"YOCTUI_BUILD_PERFORMANCE": '{"cpu":15000,"io":15000,"memory":15000}'},
        ):
            with self.assertRaisesRegex(
                self.bridge.ServerUnavailable, "native Tinfoil"
            ):
                adapter.server()

    def test_symlinked_cache_directory_is_rejected(self):
        with (
            tempfile.TemporaryDirectory() as directory,
            patch("tempfile.gettempdir", return_value=directory),
        ):
            target = Path(directory) / "target"
            target.mkdir(mode=0o700)
            (Path(directory) / f"yoctui-performance-{os.getuid()}").symlink_to(target)
            with self.assertRaisesRegex(RuntimeError, "unsafe"):
                self.params('{"cpu":15000,"io":15000,"memory":15000}')

    def test_corrupted_cached_profile_is_rejected(self):
        if not Path("/proc/pressure/memory").exists():
            self.skipTest("Linux PSI unavailable")
        with (
            tempfile.TemporaryDirectory() as directory,
            patch("tempfile.gettempdir", return_value=directory),
        ):
            value = '{"cpu":15000,"io":15000,"memory":15000}'
            path = Path(self.params(value).postfile[0])
            path.write_text("unexpected user data")
            with self.assertRaisesRegex(RuntimeError, "mismatch"):
                self.params(value)
