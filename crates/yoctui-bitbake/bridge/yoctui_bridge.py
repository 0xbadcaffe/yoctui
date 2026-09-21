#!/usr/bin/env python3
"""NDJSON BitBake bridge entry point."""

from pathlib import Path

_COMPONENTS = (
    "protocol_and_compatibility.py",
    "tinfoil_workspace.py",
    "tinfoil_metadata.py",
    "tinfoil_runtime.py",
    "adapter.py",
    "adapter_selection.py",
    "workspace_types.py",
    "dependency_types.py",
    "events.py",
    "commands.py",
    "entrypoint.py",
)
_COMPONENT_ROOT = Path(__file__).with_name("yoctui_bridge_components")
for _component in _COMPONENTS:
    _path = _COMPONENT_ROOT / _component
    exec(compile(_path.read_text(encoding="utf-8"), _path, "exec"), globals(), globals())
