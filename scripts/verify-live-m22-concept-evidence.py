#!/usr/bin/env python3
"""Reject incomplete, unsupported, stale, or unattributed M22 live evidence."""

from __future__ import annotations

import hashlib
import json
import os
import re
import struct
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = Path(
    os.environ.get(
        "YOCTUI_M22_EVIDENCE",
        ROOT / "artifacts/release-quality/m22-concept-live",
    )
).resolve()
BASE = Path(
    os.environ.get(
        "YOCTUI_M22_BASE_MANIFEST",
        ROOT / "artifacts/release-quality/next-generation-ui/manifest.json",
    )
).resolve()
SCENARIOS = {
    "idle-dashboard",
    "active-build-tasks",
    "failed-build-errors",
    "rootfs-composition",
    "editor-application-menu",
    "terminal-sessions",
}
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fail(message: str) -> None:
    raise SystemExit(f"M22 live concept evidence failed: {message}")


def safe_artifact(name: object) -> Path:
    if not isinstance(name, str) or not name or Path(name).name != name:
        fail(f"unsafe artifact name: {name!r}")
    path = EVIDENCE / name
    if not path.is_file() or not path.stat().st_size:
        fail(f"missing artifact: {name}")
    return path


def require_hash(entry: dict[str, object], key: str, path: Path) -> None:
    value = entry.get(key)
    if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
        fail(f"{key} is not a SHA-256 digest")
    if sha256(path) != value:
        fail(f"{path.name} does not match {key}")


def main() -> int:
    manifest_path = EVIDENCE / "manifest.json"
    checksums_path = EVIDENCE / "checksums.sha256"
    if not manifest_path.is_file() or not checksums_path.is_file() or not BASE.is_file():
        fail("manifest, checksums, or base supported-host manifest is missing")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    base = json.loads(BASE.read_text(encoding="utf-8"))
    if manifest.get("schema") != 1 or manifest.get("label") != "supported-live-m22-concept-parity":
        fail("manifest identity is not supported live M22 evidence")
    if "Ubuntu 24.04" not in str(manifest.get("host_distribution")):
        fail("unsupported host distribution cannot satisfy live evidence")
    if manifest.get("host_libc") != "glibc 2.39":
        fail("supported live evidence requires glibc 2.39")
    if manifest.get("poky_revision") != "d0b46a6624ec9c61c47270745dd0b2d5abbe6ac1":
        fail("evidence is not the exact official Poky yocto-5.2.4 revision")
    if manifest.get("bitbake_version") != "BitBake Build Tool Core version 2.12.1":
        fail("evidence is not the expected BitBake 2.12.1 runtime")
    if manifest.get("machine") != "qemux86-64" or manifest.get("target") != "core-image-minimal":
        fail("live machine/target identity changed")
    for key in (
        "source_commit",
        "binary_sha256",
        "host_distribution",
        "host_libc",
        "poky_revision",
        "bitbake_version",
        "machine",
        "distro",
        "yocto_release",
        "target",
        "started_utc",
        "finished_utc",
    ):
        if manifest.get(key) != base.get(key):
            fail(f"M22 and base supported-host evidence disagree on {key}")
    source_commit = manifest.get("source_commit")
    if not isinstance(source_commit, str) or not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        fail("source commit is malformed")
    ancestry = subprocess.run(
        ["git", "merge-base", "--is-ancestor", source_commit, "HEAD"], cwd=ROOT
    )
    if ancestry.returncode != 0:
        fail("live source commit is not an ancestor of HEAD")
    binary_hash = manifest.get("binary_sha256")
    if not isinstance(binary_hash, str) or not SHA256_RE.fullmatch(binary_hash):
        fail("binary hash is malformed")
    if live_binary := os.environ.get("YOCTUI_LIVE_BINARY"):
        binary = Path(live_binary).resolve()
        if not binary.is_file() or sha256(binary) != binary_hash:
            fail("YOCTUI_LIVE_BINARY does not match the evidence binary")

    scenarios = manifest.get("scenarios")
    if not isinstance(scenarios, dict) or set(scenarios) != SCENARIOS:
        fail("manifest must contain exactly the six named scenarios")
    for scenario in sorted(SCENARIOS):
        entry = scenarios[scenario]
        if not isinstance(entry, dict):
            fail(f"{scenario} entry is malformed")
        interactions = entry.get("interactions")
        assertions = entry.get("observed_assertions")
        if not isinstance(interactions, list) or not interactions or not all(
            isinstance(value, str) and value for value in interactions
        ):
            fail(f"{scenario} has no explicit interactions")
        if not isinstance(assertions, list) or not assertions or not all(
            isinstance(value, str) and value for value in assertions
        ):
            fail(f"{scenario} has no observed assertions")
        paths: dict[str, Path] = {}
        for field in ("report", "terminal", "semantic", "metadata", "raster"):
            paths[field] = safe_artifact(entry.get(field))
            require_hash(entry, f"{field}_sha256", paths[field])
        report = json.loads(paths["report"].read_text(encoding="utf-8"))
        if report.get("scenario") != scenario:
            fail(f"{scenario} report identity changed")
        if report.get("interactions") != interactions or report.get("observed_assertions") != assertions:
            fail(f"{scenario} report contract differs from the manifest")
        semantic = paths["semantic"].read_text(encoding="utf-8")
        auxiliary_name = entry.get("auxiliary_semantic")
        if auxiliary_name is not None:
            auxiliary = safe_artifact(auxiliary_name)
            require_hash(entry, "auxiliary_semantic_sha256", auxiliary)
            semantic += "\n" + auxiliary.read_text(encoding="utf-8")
        for assertion in assertions:
            if assertion not in semantic:
                fail(f"{scenario} did not render observed assertion {assertion!r}")
        meta = paths["metadata"].read_text(encoding="utf-8")
        if "width=160\n" not in meta or "height=50\n" not in meta:
            fail(f"{scenario} is not a 160x50 PTY capture")
        if b"\x1b[?1049h" not in paths["terminal"].read_bytes():
            fail(f"{scenario} terminal did not enter the alternate screen")
        header = paths["raster"].read_bytes()[:24]
        if (
            len(header) != 24
            or header[:8] != b"\x89PNG\r\n\x1a\n"
            or header[12:16] != b"IHDR"
            or struct.unpack(">II", header[16:24]) != (1600, 1000)
        ):
            fail(f"{scenario} raster is not a 1600x1000 PNG")

    expected_lines = {
        f"{sha256(path)}  {path.name}"
        for path in EVIDENCE.iterdir()
        if path.is_file() and path != checksums_path
    }
    actual_lines = set(checksums_path.read_text(encoding="utf-8").splitlines())
    if actual_lines != expected_lines:
        fail("checksums.sha256 does not exactly cover live evidence")
    print("M22 supported-host live concept evidence passed: 6 scenarios, one binary")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
