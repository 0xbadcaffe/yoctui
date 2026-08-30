#!/usr/bin/env python3
"""Require one coherent Yoctui workspace version and a bump per commit."""

from __future__ import annotations

import re
import subprocess
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SEMVER = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")


def fail(message: str) -> None:
    print(f"version policy failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def run_git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, text=True, stdout=subprocess.PIPE
    ).stdout


def parse_version(cargo_toml: bytes, source: str) -> tuple[str, tuple[int, int, int]]:
    data = tomllib.loads(cargo_toml.decode())
    value = data.get("workspace", {}).get("package", {}).get("version")
    match = SEMVER.fullmatch(value or "")
    if match is None:
        fail(f"{source} must declare a numeric workspace.package.version")
    return value, tuple(int(part) for part in match.groups())


def version_increased(current: tuple[int, int, int], previous: tuple[int, int, int]) -> bool:
    return current > previous


def baseline_cargo_toml() -> tuple[str, bytes] | None:
    revision = "HEAD" if run_git("status", "--porcelain") else "HEAD^"
    probe = subprocess.run(
        ["git", "show", f"{revision}:Cargo.toml"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    return None if probe.returncode else (revision, probe.stdout)


def dependency_tables(value: object):
    if not isinstance(value, dict):
        return
    for key, child in value.items():
        if key in {"dependencies", "dev-dependencies", "build-dependencies"}:
            yield child
        yield from dependency_tables(child)


def check_internal_versions(version: str) -> None:
    manifests = sorted((ROOT / "crates").glob("*/Cargo.toml")) + [ROOT / "fuzz/Cargo.toml"]
    mismatches: list[str] = []
    for manifest in manifests:
        data = tomllib.loads(manifest.read_text())
        for dependencies in dependency_tables(data):
            if not isinstance(dependencies, dict):
                continue
            for name, dependency in dependencies.items():
                if not name.startswith("yoctui-") or not isinstance(dependency, dict):
                    continue
                if "path" in dependency and dependency.get("version") != version:
                    mismatches.append(
                        f"{manifest.relative_to(ROOT)}: {name} uses {dependency.get('version')!r}"
                    )
    if mismatches:
        fail(f"internal dependency versions must be {version}:\n  " + "\n  ".join(mismatches))


def main() -> None:
    current, current_tuple = parse_version((ROOT / "Cargo.toml").read_bytes(), "Cargo.toml")
    baseline = baseline_cargo_toml()
    if baseline is not None:
        revision, baseline_text = baseline
        previous, previous_tuple = parse_version(baseline_text, f"{revision}:Cargo.toml")
        if not version_increased(current_tuple, previous_tuple):
            fail(f"workspace version {current} must be greater than {revision} version {previous}")
        transition = f"{previous} -> {current}"
    else:
        transition = f"initial -> {current}"

    check_internal_versions(current)
    package_script = (ROOT / "scripts/verify-cratesio-package.sh").read_text()
    if f'version="{current}"' not in package_script:
        fail("scripts/verify-cratesio-package.sh must use the workspace version")
    print(f"version policy valid: {transition}")


if __name__ == "__main__":
    main()
