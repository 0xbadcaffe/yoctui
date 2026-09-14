#!/usr/bin/env python3
"""Keep workspace library roots small and shared utilities available to each crate."""
from pathlib import Path
import tomllib

ROOT = Path(__file__).resolve().parents[1]
errors = []
for manifest in sorted((ROOT / "crates").glob("*/Cargo.toml")):
    package = tomllib.loads(manifest.read_text())
    name = package["package"]["name"]
    publication = package["package"].get("publish")
    if publication is not False and publication != ["crates-io"]:
        errors.append(f"{name} must declare the shared crates-io publication registry")
    library = manifest.parent / "src/lib.rs"
    if library.exists():
        lines = len(library.read_text().splitlines())
        if lines > 1000:
            errors.append(f"{library.relative_to(ROOT)} has {lines} lines; limit is 1000")
    if name != "yoctui-utils" and "yoctui-utils" not in package.get("dependencies", {}):
        errors.append(f"{name} must consume the shared yoctui-utils crate")
if errors:
    raise SystemExit("\n".join(errors))
print("library layout valid: roots <= 1000 lines; utilities and publication registries consistent")
