#!/usr/bin/env python3
"""Generate the structured Raw Mode catalog from the immutable reference snapshot."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shlex
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "docs/reference/bitbake-cheatsheet-wrynose-6.0-bitbake-2.18.md"
OUTPUT = ROOT / "crates/yoctui-model/src/raw_catalog_builtin.rs"
OUTPUT_DIR = ROOT / "crates/yoctui-model/src/raw_catalog_builtin"
EXPECTED_SHA256 = "ad95ecfa6a17691fa2a6d12f598f01fbd33de524c2a08ebccd218ef5fe88dd47"

PLACEHOLDER_RE = re.compile(r"<([^<>]+)>")
NUMBERED_HEADING_RE = re.compile(r"^(\d+)\.\s+(.*)$")
NON_ID_RE = re.compile(r"[^a-z0-9]+")


@dataclass
class Entry:
    line: int
    category_id: str
    heading: str
    description: str
    command: str
    executable: bool


@dataclass
class Category:
    id: str
    heading: str
    label: str
    number: int | None
    entries: list[Entry] = field(default_factory=list)


@dataclass(frozen=True)
class Parameter:
    id: str
    label: str
    placeholder: str
    kind: str


def quoted(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def slug(value: str) -> str:
    value = value.lower().replace("2.18", "2-18").replace("6.0", "6-0")
    value = NON_ID_RE.sub("-", value).strip("-")
    return value or "entry"


def is_direct_bitbake(command: str) -> bool:
    if not command.startswith("bitbake "):
        return False
    return not any(operator in command for operator in (" | ", " > ", " && ", " || ", "; "))
