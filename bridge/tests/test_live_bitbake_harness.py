"""Deterministic preflight tests; these do not claim live compatibility."""

import os
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[2] / "scripts" / "verify-live-bitbake.sh"


def run_preflight(environment):
    env = os.environ.copy()
    env.pop("BUILDDIR", None)
    env.pop("YOCTUI_LIVE_BITBAKE", None)
    env.pop("YOCTUI_LIVE_BUILD_DIR", None)
    env.pop("YOCTUI_OE_INIT_BUILD_ENV", None)
    env.update(environment)
    return subprocess.run(
        ["bash", str(SCRIPT)],
        cwd=SCRIPT.parents[1],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )


class LiveBitBakeHarnessPreflightTests(unittest.TestCase):
    def test_live_smoke_is_explicitly_opt_in(self):
        result = run_preflight({})
        self.assertEqual(result.returncode, 0)
        self.assertIn("SKIP live BitBake smoke", result.stdout)

    def test_enabled_smoke_requires_a_build_directory(self):
        result = run_preflight({"YOCTUI_LIVE_BITBAKE": "1"})
        self.assertEqual(result.returncode, 2)
        self.assertIn("YOCTUI_LIVE_BUILD_DIR is required", result.stderr)

    def test_nonexistent_build_directory_is_reported(self):
        result = run_preflight(
            {
                "YOCTUI_LIVE_BITBAKE": "1",
                "YOCTUI_LIVE_BUILD_DIR": "/definitely/not/a/yocto/build",
            }
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("build directory does not exist", result.stderr)

    def test_uninitialized_build_directory_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            result = run_preflight(
                {
                    "YOCTUI_LIVE_BITBAKE": "1",
                    "YOCTUI_LIVE_BUILD_DIR": directory,
                }
            )
        self.assertEqual(result.returncode, 2)
        self.assertIn("not an initialized build directory", result.stderr)

    def test_missing_environment_wrapper_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            conf = Path(directory, "conf")
            conf.mkdir()
            Path(conf, "bblayers.conf").touch()
            Path(conf, "local.conf").touch()
            result = run_preflight(
                {
                    "YOCTUI_LIVE_BITBAKE": "1",
                    "YOCTUI_LIVE_BUILD_DIR": directory,
                }
            )
        self.assertEqual(result.returncode, 2)
        self.assertIn("no init-build-env wrapper found", result.stderr)


if __name__ == "__main__":
    unittest.main()
