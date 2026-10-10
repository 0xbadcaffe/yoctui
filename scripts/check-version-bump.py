#!/usr/bin/env python3
"""Require coherent versions and bumps for product changes, not CI/docs."""

from __future__ import annotations

import re
import ast
import argparse
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


def product_path(path: str) -> bool:
    # Dedicated unit/integration test trees are not shipped runtime code.
    # Inline tests remain part of their production source file and still count.
    if path.startswith("crates/") and ("/src/tests/" in path or "/tests/" in path):
        return False
    return path in {"Cargo.toml", "Cargo.lock", "fuzz/Cargo.toml"} or (
        path.startswith("crates/")
        and (path.endswith("/Cargo.toml") or "/src/" in path or "/bridge/" in path)
    )


def unchanged_reference_include(previous: bytes, current: bytes, path: str, revision: str) -> bool:
    if "/src/tests/" not in path:
        return False
    include = re.compile(rb'^\s*include_str!\("([^"\\]+)"\);?[^\S\n]*$', re.M)
    before, after = include.findall(previous), include.findall(current)
    if not before or len(before) != len(after) or before == after:
        return False
    normalize = lambda match: match.group(0).replace(match[1], b"<reference>")
    if include.sub(normalize, previous) != include.sub(normalize, current):
        return False
    try:
        for old, new in zip(before, after):
            old_target = (ROOT / path).parent.joinpath(old.decode()).resolve()
            new_target = (ROOT / path).parent.joinpath(new.decode()).resolve()
            old_target.relative_to(ROOT / "docs/reference")
            new_target.relative_to(ROOT / "docs/reference")
            content = subprocess.run(
                ["git", "show", f"{revision}:{old_target.relative_to(ROOT)}"],
                cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            )
            if content.returncode or content.stdout != new_target.read_bytes():
                return False
    except (OSError, UnicodeError, ValueError):
        return False
    return True


def product_changed(paths: list[str], revision: str) -> bool:
    for path in paths:
        if not product_path(path):
            continue
        if path.endswith((".py", ".rs")):
            previous = subprocess.run(
                ["git", "show", f"{revision}:{path}"], cwd=ROOT,
                stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            )
            try:
                current = (ROOT / path).read_bytes()
                if previous.returncode == 0 and path.endswith(".rs"):
                    if unchanged_reference_include(previous.stdout, current, path, revision):
                        continue  # Test-only documentation rename; identical included bytes.
                if previous.returncode == 0 and path.endswith(".py") and ast.dump(ast.parse(previous.stdout)) == ast.dump(ast.parse(current)):
                    continue  # Formatting/comments only; runtime AST identical.
            except (OSError, SyntaxError, UnicodeError):
                pass
        return True
    return False


def baseline_cargo_toml(base_revision: str | None = None) -> tuple[str, bytes, list[str]] | None:
    changed = run_git("diff", "--name-only", "HEAD").splitlines()
    # New product files count, but unrelated untracked captures must not hide
    # the last committed product change by forcing a comparison against HEAD.
    changed += [
        path for path in run_git("ls-files", "--others", "--exclude-standard").splitlines()
        if product_path(path)
    ]
    revision = base_revision or ("HEAD" if any(product_path(path) for path in changed) else "HEAD^")
    probe = subprocess.run(
        ["git", "show", f"{revision}:Cargo.toml"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if probe.returncode:
        if base_revision is not None:
            fail(f"explicit baseline {base_revision} does not contain Cargo.toml")
        return None
    if base_revision is not None or revision == "HEAD^":
        changed += run_git("diff", "--name-only", revision, "HEAD").splitlines()
    return revision, probe.stdout, changed


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


def main(base_revision: str | None = None) -> None:
    current, current_tuple = parse_version((ROOT / "Cargo.toml").read_bytes(), "Cargo.toml")
    baseline = baseline_cargo_toml(base_revision)
    if baseline is not None:
        revision, baseline_text, changed = baseline
        previous, previous_tuple = parse_version(baseline_text, f"{revision}:Cargo.toml")
        changed_product = product_changed(changed, revision)
        if current_tuple < previous_tuple or (
            changed_product and not version_increased(current_tuple, previous_tuple)
        ):
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
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", help="Validate the entire feature branch against this Git revision")
    main(parser.parse_args().base)
