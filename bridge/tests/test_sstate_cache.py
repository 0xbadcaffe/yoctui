"""Native sstate postread settings are scoped to a reviewed build, not local.conf."""

import importlib.util
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from .support import BRIDGE


class SstateCacheTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("bridge_cache", BRIDGE)
        cls.bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.bridge)

    def profile(self, **kwargs):
        return (
            dict(
                sstate_directory="",
                downloads_directory="",
                mirror_url="",
                hash_server="auto",
                hash_upstream="",
            )
            | kwargs
        )

    def test_literal_assignment_mapping_and_blank_preservation(self):
        profile = self.profile(
            sstate_directory="/mnt/cache",
            downloads_directory="/mnt/downloads",
            mirror_url="https://host/cache",
            hash_upstream="hash:8686",
        )
        assignments = self.bridge.cache_profile_assignments(json.dumps(profile))
        for line in [
            'SSTATE_DIR = "/mnt/cache"',
            'DL_DIR = "/mnt/downloads"',
            'SSTATE_MIRRORS = "file://.* https://host/cache/PATH;downloadfilename=PATH"',
            'BB_HASHSERVE = "auto"',
            'BB_HASHSERVE_UPSTREAM = "hash:8686"',
            'BB_SIGNATURE_HANDLER = "OEEquivHash"',
        ]:
            self.assertIn(line, assignments)
        self.assertNotIn(
            "SSTATE_DIR",
            self.bridge.cache_profile_assignments(json.dumps(self.profile())),
        )
        self.assertEqual(self.bridge.cache_profile_assignments(""), "")
        for mirror in ["file:///mnt/cache/PATH", "https://[::1]:8080/PATH"]:
            assignments = self.bridge.cache_profile_assignments(
                json.dumps(self.profile(mirror_url=mirror))
            )
            self.assertIn(mirror + ";downloadfilename=PATH", assignments)
            self.assertNotIn("PATH/PATH", assignments)

    def test_invalid_values_fail_before_native_parser(self):
        invalid = [
            self.profile(sstate_directory=v)
            for v in [
                "/",
                "../cache",
                "/cache/../other",
                "${CACHE}",
                'a"\nINJECT',
                "x" * 2049,
            ]
        ]
        invalid += [
            self.profile(mirror_url=v)
            for v in [
                "https://user:secret@host/cache",
                "https://host:0/cache",
                "http://host/PATH/other",
                "http://host/PATH/PATH",
                "file:///../cache",
            ]
        ]
        invalid += [
            self.profile(hash_server=v)
            for v in [
                "host",
                "host:0",
                "host:65536",
                "user@host:8686",
                "unix:///../sock",
            ]
        ]
        invalid += [
            self.profile(hash_server="remote:8686", hash_upstream="other:8686"),
            self.profile(hash_server=""),
            {},
            [],
            self.profile(extra="unknown"),
        ]
        for profile in invalid:
            with self.subTest(profile=profile), self.assertRaises(RuntimeError):
                self.bridge.cache_profile_assignments(json.dumps(profile))

    def test_cache_only_does_not_require_psi_and_combines_with_pressure_in_one_file(
        self,
    ):
        with (
            tempfile.TemporaryDirectory() as directory,
            patch("tempfile.gettempdir", return_value=directory),
        ):
            tinfoil = SimpleNamespace(
                TinfoilConfigParameters=lambda **kwargs: SimpleNamespace(**kwargs)
            )
            module = SimpleNamespace(__version__="2.20.0")
            environment = {
                "YOCTUI_SSTATE_PROFILE": json.dumps(self.profile()),
                "YOCTUI_BUILD_PERFORMANCE": "",
            }
            with patch.dict(os.environ, environment):
                params = self.bridge.performance_config_params(module, tinfoil)
                path = Path(params.postfile[0])
                text = path.read_text()
                self.assertIn('BB_HASHSERVE = "auto"', text)
                self.assertNotIn("BB_PRESSURE", text)
                self.assertEqual(
                    self.bridge.performance_config_params(module, tinfoil).postfile,
                    params.postfile,
                )
                if Path("/proc/pressure/memory").exists():
                    with patch.dict(
                        os.environ,
                        {
                            "YOCTUI_BUILD_PERFORMANCE": json.dumps(
                                dict(cpu=1, io=2, memory=3)
                            )
                        },
                    ):
                        combined = self.bridge.performance_config_params(
                            module, tinfoil
                        )
                        self.assertEqual(len(combined.postfile), 1)
                        self.assertIn(
                            'BB_PRESSURE_MAX_MEMORY = "3"',
                            Path(combined.postfile[0]).read_text(),
                        )
                        self.assertIn(
                            'BB_HASHSERVE = "auto"',
                            Path(combined.postfile[0]).read_text(),
                        )
            with patch.dict(
                os.environ,
                {"YOCTUI_SSTATE_PROFILE": "", "YOCTUI_BUILD_PERFORMANCE": ""},
            ):
                self.assertIsNone(
                    self.bridge.performance_config_params(module, tinfoil)
                )

    def test_alternative_connector_rejects_cache_profile_instead_of_ignoring_it(self):
        adapter = self.bridge.BitBakeAdapter(
            "2.20.0", SimpleNamespace(server=SimpleNamespace(connect=lambda: object()))
        )
        with patch.dict(
            os.environ,
            {
                "YOCTUI_BUILD_PERFORMANCE": "",
                "YOCTUI_SSTATE_PROFILE": json.dumps(self.profile()),
            },
        ):
            with self.assertRaisesRegex(
                self.bridge.ServerUnavailable, "native Tinfoil"
            ):
                adapter.server()
