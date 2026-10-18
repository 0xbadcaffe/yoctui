#!/usr/bin/env python3
"""Deterministic public-CLI stand-in; never a real BitBake build tool."""

import json
from pathlib import Path
import shlex
import sys

arguments = sys.argv[1:]
settings = {}
while arguments and arguments[0] == "--setting":
    settings[arguments[2]] = arguments[3]
    arguments = arguments[4:]
if arguments[:2] == ["--color", "never"]:
    arguments = arguments[2:]
if arguments[:1] == ["--no-network"]:
    arguments = arguments[1:]
operation = arguments[0]
if operation in ("status", "update"):
    setup = Path(arguments[arguments.index("--setup-dir") + 1])
    if operation == "status":
        counter = setup / "config/status-reads"
        count = int(counter.read_text()) + 1 if counter.exists() else 1
        counter.write_text(str(count))
        print(f"SETUP_STATUS {count}: synchronized local fixture")
    else:
        assert arguments[arguments.index("--rebase-conflicts-strategy") + 1] == "abort"
        policy = arguments[arguments.index("--update-bb-conf") + 1]
        if policy == "prompt":
            assert input("Apply fixture configuration? (y/N): ") == "y"
        (setup / "config/update-complete").write_text(policy)
        print("SETUP_UPDATE_COMPLETE")
    sys.exit(0)

assert arguments[:2] == ["init", "--no-init-vscode"]
name = arguments[arguments.index("--setup-dir-name") + 1]
top = Path(settings["top-dir-prefix"]) / settings["top-dir-name"]
assert input("Choose fixture machine: ") == "qemux86-64"
build = top / name / "build"
(build / "conf").mkdir(parents=True)
(build / "conf/local.conf").write_text('MACHINE = "qemux86-64"\n')
(build / "conf/bblayers.conf").write_text('BBLAYERS = ""\n')
bin_dir = Path(__file__).parent / "bin"
(build / "init-build-env").write_text(
    f'test "$#" = 0 || return 41\nexport PATH={shlex.quote(str(bin_dir))}:"$PATH"\n'
    f"export BUILDDIR={shlex.quote(str(build))}\ncd {shlex.quote(str(build))}\n"
)
config = top / name / "config"
config.mkdir()
(config / "config-upstream.json").write_text(json.dumps({"fixture": True}))
print("SETUP_FIXTURE_COMPLETE", flush=True)
