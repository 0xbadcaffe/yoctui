#!/usr/bin/env python3
"""Enforce the repository's source-module and workspace-library boundaries."""
from __future__ import annotations

from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]
SOURCE_ROOTS = ("crates", "bridge", "scripts", "fuzz")
SOURCE_SUFFIXES = frozenset({".rs", ".py", ".sh"})
MAX_SOURCE_LINES = 500
INLINE_RUST_TEST_MODULE = re.compile(
    r"#\[cfg\(test\)\](?:\s*#\[[^\n]+\])*\s*mod\s+[A-Za-z_][A-Za-z0-9_]*\s*\{"
)
RUST_TEST_BODY = re.compile(r"#\s*\[\s*test\s*\]")


def maintained_sources(root: Path) -> list[Path]:
    sources: list[Path] = []
    for relative_root in SOURCE_ROOTS:
        source_root = root / relative_root
        if not source_root.exists():
            continue
        sources.extend(
            path
            for path in source_root.rglob("*")
            if path.is_file()
            and path.suffix in SOURCE_SUFFIXES
            and "__pycache__" not in path.parts
        )
    return sorted(sources)


def source_layout_errors(root: Path, sources: list[Path]) -> list[str]:
    errors: list[str] = []
    for source in sources:
        relative = source.relative_to(root)
        text = source.read_text(encoding="utf-8")
        lines = len(text.splitlines())
        if lines > MAX_SOURCE_LINES:
            errors.append(
                f"{relative} has {lines} lines; source limit is {MAX_SOURCE_LINES}"
            )
        if source.suffix != ".rs":
            continue
        if INLINE_RUST_TEST_MODULE.search(text):
            errors.append(f"{relative} contains an inline Rust test module")
        if "tests" not in relative.parts and RUST_TEST_BODY.search(text):
            errors.append(f"{relative} contains a test body outside a tests folder")
    return errors


def workspace_manifest_errors(root: Path) -> list[str]:
    errors: list[str] = []
    for manifest in sorted((root / "crates").glob("*/Cargo.toml")):
        package = tomllib.loads(manifest.read_text(encoding="utf-8"))
        name = package["package"]["name"]
        publication = package["package"].get("publish")
        if publication is not False and publication != ["crates-io"]:
            errors.append(
                f"{name} must declare the shared crates-io publication registry"
            )
        if name != "yoctui-utils" and "yoctui-utils" not in package.get(
            "dependencies", {}
        ):
            errors.append(f"{name} must consume the shared yoctui-utils crate")
    return errors


def main() -> None:
    sources = maintained_sources(ROOT)
    errors = workspace_manifest_errors(ROOT) + source_layout_errors(ROOT, sources)
    if errors:
        raise SystemExit("\n".join(errors))
    largest = max((len(path.read_text(encoding="utf-8").splitlines()) for path in sources), default=0)
    print(
        f"library layout valid: {len(sources)} sources <= {MAX_SOURCE_LINES} lines "
        f"(largest {largest}); Rust tests use test folders; utilities and publication "
        "registries consistent"
    )


if __name__ == "__main__":
    main()
