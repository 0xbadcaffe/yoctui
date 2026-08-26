#!/usr/bin/env python3
"""Generate and verify Yoctui's lockfile notices and CycloneDX inventories."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tomllib
import uuid
from collections import defaultdict
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
AUDIT_PATH = REPO_ROOT / "docs/compliance/widget-candidates.toml"
CANDIDATE_SBOM_PATH = REPO_ROOT / "docs/compliance/widget-candidates.cdx.json"
NOTICE_PATH = REPO_ROOT / "docs/compliance/THIRD_PARTY_NOTICES.md"
SBOM_PATH = REPO_ROOT / "docs/compliance/yoctui.cdx.json"
EXPECTED_CANDIDATES = {
    "ratatui-image",
    "ratatui-textarea",
    "throbber-widgets-tui",
    "tui-big-text",
    "tui-checkbox",
    "tui-logger",
    "tui-menu",
    "tui-nodes",
    "tui-piechart",
    "tui-scrollview",
    "tui-term",
    "tui-tree-widget",
    "tui-widget-list",
}
LICENSE_PATTERNS = ("LICENSE*", "LICENCE*", "COPYING*", "NOTICE*", "COPYRIGHT*")
HEX_64 = re.compile(r"^[0-9a-f]{64}$")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_lock(path: Path) -> dict[tuple[str, str, str], dict[str, Any]]:
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    result = {}
    for package in data["package"]:
        source = package.get("source", "local")
        result[(package["name"], package["version"], source)] = package
    return result


def cargo_metadata(manifest: Path | None = None) -> dict[str, Any]:
    command = ["cargo", "metadata", "--format-version", "1", "--locked", "--offline"]
    if manifest is not None:
        command.extend(["--manifest-path", str(manifest)])
    return json.loads(subprocess.check_output(command, cwd=REPO_ROOT))


def package_ref(package: dict[str, Any]) -> str:
    return f"pkg:cargo/{package['name']}@{package['version']}"


def third_party_packages(metadata: dict[str, Any]) -> list[dict[str, Any]]:
    workspace = set(metadata["workspace_members"])
    return sorted(
        (package for package in metadata["packages"] if package["id"] not in workspace),
        key=lambda package: (package["name"], package["version"], package["id"]),
    )


def component_for(package: dict[str, Any], lock: dict[tuple[str, str, str], dict[str, Any]]) -> dict[str, Any]:
    source = package.get("source") or "local"
    lock_package = lock.get((package["name"], package["version"], source), {})
    component: dict[str, Any] = {
        "type": "library",
        "bom-ref": package_ref(package),
        "name": package["name"],
        "version": package["version"],
        "purl": package_ref(package),
        "licenses": [{"expression": package.get("license") or "NOASSERTION"}],
        "properties": [{"name": "cargo:source", "value": source}],
    }
    if package.get("authors"):
        component["authors"] = [{"name": author} for author in package["authors"]]
    checksum = lock_package.get("checksum")
    if checksum:
        component["hashes"] = [{"alg": "SHA-256", "content": checksum}]
    references = []
    if package.get("repository"):
        references.append({"type": "vcs", "url": package["repository"]})
    references.append(
        {
            "type": "distribution",
            "url": f"https://crates.io/crates/{package['name']}/{package['version']}",
        }
    )
    component["externalReferences"] = references
    return component


def build_sbom(metadata: dict[str, Any], lock_path: Path, *, candidate_names: set[str] | None = None) -> dict[str, Any]:
    lock = load_lock(lock_path)
    packages = third_party_packages(metadata)
    third_ids = {package["id"] for package in packages}
    by_id = {package["id"]: package for package in metadata["packages"]}
    components = [component_for(package, lock) for package in packages]
    if candidate_names is not None:
        for component in components:
            if component["name"] in candidate_names:
                component["properties"].append({"name": "yoctui:candidate-root", "value": "true"})
    dependencies = []
    for node in metadata["resolve"]["nodes"]:
        if node["id"] not in third_ids:
            continue
        dependencies.append(
            {
                "ref": package_ref(by_id[node["id"]]),
                "dependsOn": sorted(
                    package_ref(by_id[dependency["pkg"]])
                    for dependency in node["deps"]
                    if dependency["pkg"] in third_ids
                ),
            }
        )
    dependencies.sort(key=lambda item: item["ref"])
    lock_hash = sha256(lock_path.read_bytes())
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "serialNumber": f"urn:uuid:{uuid.uuid5(uuid.NAMESPACE_URL, lock_hash)}",
        "version": 1,
        "metadata": {
            "tools": {"components": [{"type": "application", "name": "yoctui-third-party-compliance", "version": "1"}]},
            "properties": [{"name": "yoctui:cargo-lock-sha256", "value": lock_hash}],
        },
        "components": components,
        "dependencies": dependencies,
    }


def notice_materials(package: dict[str, Any]) -> list[tuple[str, bytes]]:
    root = Path(package["manifest_path"]).parent
    paths: set[Path] = set()
    for pattern in LICENSE_PATTERNS:
        paths.update(path for path in root.glob(pattern) if path.is_file())
    return [(path.name, path.read_bytes()) for path in sorted(paths)]


def build_notices(metadata: dict[str, Any], lock_path: Path) -> str:
    packages = third_party_packages(metadata)
    lock = load_lock(lock_path)
    materials: dict[str, bytes] = {}
    material_names: dict[str, set[str]] = defaultdict(set)
    material_packages: dict[str, set[str]] = defaultdict(set)
    package_materials: dict[str, list[str]] = {}
    for package in packages:
        identity = f"{package['name']} {package['version']}"
        hashes = []
        for filename, content in notice_materials(package):
            digest = sha256(content)
            materials[digest] = content
            material_names[digest].add(filename)
            material_packages[digest].add(identity)
            hashes.append(digest)
        package_materials[identity] = sorted(set(hashes))

    lines = [
        "# Yoctui Third-Party Notices",
        "",
        "This file is generated from the exact `Cargo.lock` graph. It inventories every non-workspace package, records the byte-authoritative SHA-256 of every packaged root-level license, notice, copying, and copyright file, and displays a Markdown-safe normalization of its content (UTF-8 text directly; non-UTF-8 data as hexadecimal). Packages with no packaged notice file remain listed with their manifest SPDX expression and authorship metadata in the SBOM.",
        "",
        f"- Cargo.lock SHA-256: `{sha256(lock_path.read_bytes())}`",
        f"- Third-party packages: {len(packages)}",
        f"- Unique packaged notice materials: {len(materials)}",
        "",
        "## Package inventory",
        "",
        "| Package | SPDX expression | Source | Checksum | Notice materials |",
        "|---|---|---|---|---|",
    ]
    for package in packages:
        source = package.get("source") or "local"
        lock_package = lock.get((package["name"], package["version"], source), {})
        checksum = lock_package.get("checksum", "not supplied")
        identity = f"{package['name']} {package['version']}"
        refs = ", ".join(f"[`{digest[:12]}`](#notice-{digest})" for digest in package_materials[identity]) or "none packaged"
        lines.append(
            f"| `{identity}` | `{package.get('license') or 'NOASSERTION'}` | `{source}` | `{checksum}` | {refs} |"
        )

    lines.extend(["", "## Packaged notice materials", ""])
    for digest in sorted(materials):
        content = materials[digest]
        names = ", ".join(f"`{name}`" for name in sorted(material_names[digest]))
        owners = ", ".join(f"`{name}`" for name in sorted(material_packages[digest]))
        lines.extend(
            [
                f'<a id="notice-{digest}"></a>',
                f"### SHA-256 `{digest}`",
                "",
                f"Packaged filenames: {names}",
                "",
                f"Used by: {owners}",
                "",
            ]
        )
        try:
            text = content.decode("utf-8")
            display = "\n".join(line.rstrip() for line in text.splitlines())
            lines.extend(["```text", display, "```", ""])
        except UnicodeDecodeError:
            lines.extend(["```text", content.hex(), "```", ""])
    return "\n".join(lines).rstrip() + "\n"


def json_bytes(value: dict[str, Any]) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def compare(path: Path, expected: bytes) -> None:
    actual = path.read_bytes() if path.exists() else b""
    if actual != expected:
        raise ValueError(f"generated compliance artifact is stale: {path.relative_to(REPO_ROOT)}")


def validate_candidate_audit(audit_path: Path, sbom_path: Path, repo_root: Path = REPO_ROOT) -> None:
    audit = tomllib.loads(audit_path.read_text(encoding="utf-8"))
    candidates = audit.get("candidate", [])
    names = {candidate.get("name") for candidate in candidates}
    if names != EXPECTED_CANDIDATES or len(candidates) != len(EXPECTED_CANDIDATES):
        raise ValueError(f"candidate set mismatch: {sorted(names ^ EXPECTED_CANDIDATES)}")
    decisions = {"adopt", "adapt", "defer", "reject"}
    by_name = {}
    for candidate in candidates:
        name = candidate["name"]
        by_name[name] = candidate
        if candidate.get("decision") not in decisions:
            raise ValueError(f"{name}: invalid decision")
        if candidate.get("admitted") and candidate.get("decision") != "adopt":
            raise ValueError(f"{name}: only adopt decisions may be admitted")
        if candidate.get("admitted") and candidate.get("msrv") == "unknown":
            raise ValueError(f"{name}: an admitted candidate must declare or prove an MSRV")
        if not HEX_64.fullmatch(candidate.get("checksum", "")):
            raise ValueError(f"{name}: invalid crate checksum")
        if not HEX_64.fullmatch(candidate.get("transitive_sha256", "")):
            raise ValueError(f"{name}: invalid transitive graph checksum")
        if candidate.get("default_features") is not False:
            raise ValueError(f"{name}: default features must be disabled and selected explicitly")
        if not candidate.get("reason") or not candidate.get("owner_task"):
            raise ValueError(f"{name}: decision needs a reason and owner task")
        requirement = candidate.get("ratatui_requirement", "")
        if not ("^0.30" in requirement or ("ratatui-core ^0.1" in requirement and "ratatui-widgets ^0.3" in requirement)):
            raise ValueError(f"{name}: not compatible with the Ratatui 0.30 package family")

    sbom = json.loads(sbom_path.read_text(encoding="utf-8"))
    if sbom.get("bomFormat") != "CycloneDX" or sbom.get("specVersion") != "1.5":
        raise ValueError("candidate SBOM is not CycloneDX 1.5")
    metadata_properties = {
        item["name"]: item["value"] for item in sbom.get("metadata", {}).get("properties", [])
    }
    lock_hash = metadata_properties.get("yoctui:cargo-lock-sha256", "")
    if not HEX_64.fullmatch(lock_hash):
        raise ValueError("candidate SBOM has no valid audit lock checksum")
    if sbom.get("serialNumber") != f"urn:uuid:{uuid.uuid5(uuid.NAMESPACE_URL, lock_hash)}":
        raise ValueError("candidate SBOM serial number does not match its audit lock")
    components = {component["bom-ref"]: component for component in sbom.get("components", [])}
    dependencies = {item["ref"]: item.get("dependsOn", []) for item in sbom.get("dependencies", [])}
    if set(dependencies) != set(components):
        raise ValueError("candidate SBOM dependency nodes do not match components")
    for ref, children in dependencies.items():
        dangling = set(children) - set(components)
        if dangling:
            raise ValueError(f"candidate SBOM has dangling dependencies for {ref}: {sorted(dangling)}")
    candidate_components = {
        component["name"]: component
        for component in components.values()
        if {item["name"]: item["value"] for item in component.get("properties", [])}.get("yoctui:candidate-root") == "true"
    }
    if set(candidate_components) != EXPECTED_CANDIDATES:
        raise ValueError("candidate SBOM roots do not match the audit")
    for name, candidate in by_name.items():
        component = candidate_components[name]
        if component["version"] != candidate["version"]:
            raise ValueError(f"{name}: SBOM version mismatch")
        hashes = {item["alg"]: item["content"] for item in component.get("hashes", [])}
        if hashes.get("SHA-256") != candidate["checksum"]:
            raise ValueError(f"{name}: SBOM checksum mismatch")
        root_ref = component["bom-ref"]
        seen: set[str] = set()
        stack = [root_ref]
        while stack:
            current = stack.pop()
            if current in seen:
                continue
            seen.add(current)
            stack.extend(dependencies[current])
        lines = []
        for ref in sorted(seen, key=lambda item: (components[item]["name"], components[item]["version"], item)):
            item = components[ref]
            properties = {prop["name"]: prop["value"] for prop in item.get("properties", [])}
            lines.append(f"{item['name']} {item['version']} {properties['cargo:source']}")
        digest = sha256(("\n".join(lines) + "\n").encode())
        if len(seen) != candidate["transitive_closure"] or digest != candidate["transitive_sha256"]:
            raise ValueError(f"{name}: transitive graph evidence mismatch")

    manifests = [repo_root / "Cargo.toml", *(repo_root / "crates").glob("*/Cargo.toml")]
    manifest_text = "\n".join(path.read_text(encoding="utf-8") for path in manifests)
    lock_text = (repo_root / "Cargo.lock").read_text(encoding="utf-8")
    for candidate in candidates:
        name = candidate["name"]
        present = re.search(rf'(?m)^name = "{re.escape(name)}"$', lock_text) is not None
        declared = re.search(rf'(?m)^\s*{re.escape(name)}\s*=', manifest_text) is not None
        if not candidate["admitted"] and (present or declared):
            raise ValueError(f"{name}: non-admitted candidate entered the workspace graph")
        if candidate["admitted"] and not (present and declared):
            raise ValueError(f"{name}: admitted candidate is absent from the workspace graph")


def write_candidate_sbom(manifest: Path) -> None:
    metadata = cargo_metadata(manifest)
    audit = tomllib.loads(AUDIT_PATH.read_text(encoding="utf-8"))
    names = {candidate["name"] for candidate in audit["candidate"]}
    sbom = build_sbom(metadata, manifest.parent / "Cargo.lock", candidate_names=names)
    CANDIDATE_SBOM_PATH.write_bytes(json_bytes(sbom))


def verify_shipped(write: bool) -> None:
    metadata = cargo_metadata()
    notices = build_notices(metadata, REPO_ROOT / "Cargo.lock").encode()
    sbom = json_bytes(build_sbom(metadata, REPO_ROOT / "Cargo.lock"))
    if write:
        NOTICE_PATH.write_bytes(notices)
        SBOM_PATH.write_bytes(sbom)
    else:
        compare(NOTICE_PATH, notices)
        compare(SBOM_PATH, sbom)
    print(f"third-party compliance valid: {len(third_party_packages(metadata))} locked packages")


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    shipped = subparsers.add_parser("shipped")
    shipped.add_argument("--write", action="store_true")
    candidates = subparsers.add_parser("candidates")
    candidates.add_argument("--audit", type=Path, default=AUDIT_PATH)
    candidates.add_argument("--sbom", type=Path, default=CANDIDATE_SBOM_PATH)
    candidate_sbom = subparsers.add_parser("write-candidate-sbom")
    candidate_sbom.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "shipped":
            verify_shipped(args.write)
        elif args.command == "candidates":
            validate_candidate_audit(args.audit, args.sbom)
            print(f"widget candidate audit valid: {len(EXPECTED_CANDIDATES)} candidates")
        else:
            write_candidate_sbom(args.manifest)
    except (KeyError, OSError, ValueError, subprocess.CalledProcessError, tomllib.TOMLDecodeError, json.JSONDecodeError) as error:
        print(f"third-party compliance failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
