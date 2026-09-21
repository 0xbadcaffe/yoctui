#!/usr/bin/env python3
"""Stable command-line entry point; implementation lives in named modules."""

from pathlib import Path

_COMPONENTS = (
    "model.py",
    "reference_parser.py",
    "command_mapping.py",
    "rust_rendering.py",
    "file_generation.py",
    "entrypoint.py",
)
_COMPONENT_ROOT = Path(__file__).with_name("yoctui_tools") / "raw_catalog"
for _component in _COMPONENTS:
    _path = _COMPONENT_ROOT / _component
    exec(compile(_path.read_text(encoding="utf-8"), _path, "exec"), globals(), globals())
