#!/usr/bin/env python3
"""Deterministic fragment fixture using the public tool's output/argv format."""

import os
from pathlib import Path
import sys

arguments = sys.argv[1:]
assert arguments[:2] == ["--color", "never"]
arguments = arguments[2:]
operation = arguments[0]
conf = Path(os.environ["BUILDDIR"]) / "conf/toolcfg.conf"
target = "core/yocto/monitor-disk-space"
enabled = target in conf.read_text() if conf.exists() else False
if operation == "list-fragments":
    print("Available built-in fragments:\nmachine/...\tSets MACHINE\n")
    print("Enabled built-in fragments:\nmachine/qemux86-64\tSelected machine\n")
    print("Available fragments in core layer located in /fixture:\n")
    print("Enabled fragments:" if enabled else "Unused fragments:")
    print(f"{target}\tMonitor disk space\n")
elif operation == "show-fragment":
    print("# Fixture fragment detail\nBB_DISKMON_DIRS = 'fixture'\n")
else:
    assert arguments[1] == target
    assert Path(arguments[arguments.index("--confpath") + 1]) == conf
    assert operation in ("enable-fragment", "disable-fragment")
    conf.write_text(
        f'OE_FRAGMENTS += "{target if operation == "enable-fragment" else ""}"\n'
    )
    print(f"Fragment {target}: {operation}")
