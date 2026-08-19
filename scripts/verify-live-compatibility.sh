#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mode="${1:-}"
case "$mode" in
  latest|older) kinds=("$mode") ;;
  --evidence-only) kinds=(latest older) ;;
  *) printf 'usage: %s {latest|older|--evidence-only}\n' "$0" >&2; exit 2 ;;
esac

python3 - "${kinds[@]}" <<'PY'
from __future__ import annotations

import datetime as dt
from pathlib import Path
import re
import subprocess
import sys
import tomllib
from urllib.parse import urlparse

ROOT = Path.cwd()
TODAY = dt.date.today()
HEX40 = re.compile(r"[0-9a-f]{40}\Z")
HEX64 = re.compile(r"[0-9a-f]{64}\Z")
RELEASE = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+\Z")
YEAR_MONTH = re.compile(r"[0-9]{4}-[0-9]{2}\Z")
OFFICIAL_HOSTS = {
    "docs.yoctoproject.org",
    "wiki.yoctoproject.org",
    "git.yoctoproject.org",
    "git.openembedded.org",
    "github.com",
}
COMMON_WORKFLOWS = {
    "environment_identity=passed",
    "capability_probe=passed",
    "doctor=passed",
    "workspace_inspection=passed",
    "recipes=passed",
    "layers=passed",
    "configuration=passed",
    "core_build_events=passed",
    "build_cancellation=passed",
    "devtool_capabilities=passed",
    "utility_capabilities=passed",
    "modern_bitbake_commands=passed",
}
COMMON_COMMANDS = {
    "bitbake.version=passed",
    "bitbake.help_options=passed",
    "bitbake_getvar.value=passed",
    "yoctui.doctor_json=passed",
    "yoctui.inspect=passed",
    "yoctui.recipes=passed",
    "yoctui.layers=passed",
    "yoctui.config=passed",
    "yoctui.daemon_build=passed",
    "yoctui.daemon_cancel=passed",
}


def fail(kind: str, message: str) -> None:
    raise SystemExit(f"live compatibility evidence ({kind}): {message}")


def require_text(data: dict, kind: str, name: str) -> str:
    value = data.get(name)
    if not isinstance(value, str) or not value.strip() or any(c in value for c in "\r\n\0"):
        fail(kind, f"{name} must be non-empty bounded text")
    return value


def require_url(data: dict, kind: str, name: str) -> str:
    value = require_text(data, kind, name)
    parsed = urlparse(value)
    if parsed.scheme != "https" or parsed.hostname not in OFFICIAL_HOSTS:
        fail(kind, f"{name} is not an allowlisted authoritative HTTPS source")
    return value


def require_unique_strings(data: dict, kind: str, name: str) -> set[str]:
    value = data.get(name)
    if not isinstance(value, list) or not value or not all(isinstance(item, str) and item for item in value):
        fail(kind, f"{name} must be a non-empty string array")
    if len(value) != len(set(value)):
        fail(kind, f"{name} contains duplicates")
    return set(value)


def verify(kind: str) -> None:
    path = ROOT / "docs" / "compatibility-evidence" / f"{kind}.toml"
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        fail(kind, f"record is missing or invalid TOML: {exc}")

    if data.get("schema_version") != 1 or data.get("kind") != kind:
        fail(kind, "schema_version/kind identity is invalid")
    if data.get("evidence_level") != "live" or data.get("fixture_only") is not False:
        fail(kind, "fixtures or non-live records cannot satisfy this gate")
    if data.get("synthetic") is not False or data.get("official_checkout") is not True:
        fail(kind, "record must identify a non-synthetic official checkout")
    if data.get("release_role") != ("latest_published_stable" if kind == "latest" else "older_supported"):
        fail(kind, "release role is missing or inconsistent")

    try:
        observed = dt.date.fromisoformat(require_text(data, kind, "observed_at"))
        expiry = int(data["expires_after_days"])
    except (KeyError, TypeError, ValueError) as exc:
        fail(kind, f"date/expiry policy is invalid: {exc}")
    if expiry < 1 or expiry > 180 or observed > TODAY or (TODAY - observed).days > expiry:
        fail(kind, "record is stale, future-dated, or has an unsafe expiry interval")

    for name in (
        "official_source_url",
        "official_release_calendar_url",
        "official_release_notes_url",
        "official_support_policy_url",
        "repository_url",
        "oe_core_repository_url",
        "bitbake_repository_url",
        "meta_yocto_repository_url",
    ):
        require_url(data, kind, name)
    composition = require_text(data, kind, "source_composition")
    checkout = require_text(data, kind, "source_checkout")
    if composition == "split_components":
        if data.get("poky_commit") != data.get("meta_yocto_commit"):
            fail(kind, "split composition poky_commit must identify the exact meta-yocto revision")
        if checkout != "fresh_official_component_checkouts":
            fail(kind, "split composition must identify fresh official component checkouts")
    elif composition == "poky_checkout":
        if checkout != "fresh_official_poky_checkout":
            fail(kind, "Poky composition must identify a fresh official Poky checkout")
    else:
        fail(kind, "source_composition is unsupported")
    for name in ("poky_commit", "oe_core_commit", "bitbake_commit", "meta_yocto_commit", "yoctui_commit"):
        if not HEX40.fullmatch(require_text(data, kind, name)):
            fail(kind, f"{name} must be an exact lowercase Git commit")
    if not HEX64.fullmatch(require_text(data, kind, "build_identity")):
        fail(kind, "build_identity must be an exact SHA-256 fingerprint")
    if not RELEASE.fullmatch(require_text(data, kind, "yocto_release")):
        fail(kind, "yocto_release must be an exact point release")
    if require_text(data, kind, "support_status") != "maintained_lts":
        fail(kind, "release must be maintained under the recorded support policy")
    if not YEAR_MONTH.fullmatch(require_text(data, kind, "support_until")):
        fail(kind, "support_until must identify an exact policy month")
    for name in (
        "yocto_series", "bitbake_version", "distro", "machine", "backend",
        "protocol_version", "host_identity",
    ):
        require_text(data, kind, name)

    commands = require_unique_strings(data, kind, "commands")
    capabilities = require_unique_strings(data, kind, "capabilities")
    workflows = require_unique_strings(data, kind, "workflows")
    if not COMMON_COMMANDS <= commands:
        fail(kind, "required live command checks are absent: " + ", ".join(sorted(COMMON_COMMANDS - commands)))
    if not COMMON_WORKFLOWS <= workflows:
        fail(kind, "required live workflows are absent: " + ", ".join(sorted(COMMON_WORKFLOWS - workflows)))
    required_capability_prefixes = {
        "bitbake.build=", "bitbake.cancellation=", "bitbake.getvar=",
        "bitbake.recipe_inventory=", "bitbake.layer_inventory=", "devtool.upgrade=",
        "recipetool.create=", "bitbake_layers.show_layers=", "pkgdata.lookup_pkg=",
    }
    for prefix in required_capability_prefixes:
        matching = [item for item in capabilities if item.startswith(prefix)]
        if len(matching) != 1:
            fail(kind, f"capability result {prefix} must occur exactly once")

    metrics = data.get("metrics")
    if not isinstance(metrics, dict):
        fail(kind, "metrics table is missing")
    if int(metrics.get("recipe_count", 0)) < 100 or int(metrics.get("layer_count", 0)) < 1:
        fail(kind, "workspace inventory metrics are implausibly small")
    if int(metrics.get("native_event_count", 0)) < 1 or int(metrics.get("successful_build_exit_code", -1)) != 0:
        fail(kind, "successful build/native-event evidence is missing")
    if float(metrics.get("cancellation_seconds", 9999)) > 15:
        fail(kind, "cancellation exceeded the bounded live policy")

    tested = data["yoctui_commit"]
    exists = subprocess.run(
        ["git", "cat-file", "-e", f"{tested}^{{commit}}"], cwd=ROOT,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    ).returncode == 0
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", tested, "HEAD"], cwd=ROOT,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    ).returncode == 0
    if not exists or not ancestor:
        fail(kind, "tested Yoctui commit is absent or not an ancestor of HEAD")

    print(
        f"live compatibility evidence valid: {kind} {data['yocto_release']} "
        f"({data['yocto_series']}) BitBake {data['bitbake_version']} observed {observed}"
    )


for requested in sys.argv[1:]:
    verify(requested)
PY
