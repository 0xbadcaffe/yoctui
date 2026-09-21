"""Bridge framing tests; compatible with both unittest and pytest collection."""

import json
import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch
from pathlib import Path


BRIDGE = Path(__file__).parents[2] / "crates/yoctui-bitbake/bridge/yoctui_bridge.py"
MAX_LINE_BYTES = 1024 * 1024
MAX_NATIVE_EVENTS_PER_POLL = 64


def run_bridge(
    *lines: bytes, environment: dict[str, str] | None = None
) -> subprocess.CompletedProcess[bytes]:
    env = os.environ.copy()
    if environment:
        env.update(environment)
    return subprocess.run(
        [sys.executable, str(BRIDGE)],
        input=b"".join(line + b"\n" for line in lines),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
        check=False,
    )
