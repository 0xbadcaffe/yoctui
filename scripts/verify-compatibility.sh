#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mode="${1:-}"
if [[ -n "$mode" && "$mode" != "--structure-only" ]]; then
  printf 'usage: %s [--structure-only]\n' "$0" >&2
  exit 2
fi

python3 - "$mode" <<'PY'
from __future__ import annotations

import datetime as dt
from pathlib import Path
import sys
import tomllib

mode = sys.argv[1]
root = Path.cwd()
registry_path = root / "docs/task-registry.toml"
try:
    registry = tomllib.loads(registry_path.read_text(encoding="utf-8"))
except (OSError, tomllib.TOMLDecodeError) as exc:
    raise SystemExit(f"compatibility gate: task registry does not parse: {exc}")

required_ids = {
    "COMPAT-SPEC-001", "COMPAT-ENV-ID-001", "COMPAT-CAP-MODEL-001",
    "COMPAT-CATALOG-001", "COMPAT-PROBE-001", "COMPAT-VERSION-001",
    "COMPAT-BITBAKE-CMD-001", "COMPAT-BITBAKE-API-001",
    "COMPAT-DEVTOOL-001", "COMPAT-RECIPETOOL-001", "COMPAT-LAYERS-001",
    "COMPAT-PKGDATA-001", "COMPAT-UTILITIES-001", "COMPAT-WORKSPACE-001",
    "COMPAT-UI-001", "COMPAT-DOCTOR-001", "COMPAT-CACHE-001",
    "COMPAT-DAEMON-001", "COMPAT-PROTOCOL-001", "COMPAT-UNKNOWN-001",
    "COMPAT-OLD-001", "COMPAT-MATRIX-001", "COMPAT-TEST-FIXTURES-001",
    "COMPAT-TEST-CMDS-001", "COMPAT-TEST-UI-001",
    "COMPAT-BITBAKE-GETVAR-001",
    "COMPAT-LIVE-LATEST-001", "COMPAT-LIVE-OLDER-001",
    "COMPAT-LIVE-MATRIX-001", "COMPAT-CI-001", "COMPAT-DOC-001",
    "COMPAT-001",
}
tasks = {task.get("id"): task for task in registry.get("task", [])}
missing = sorted(required_ids - tasks.keys())
if missing:
    raise SystemExit("compatibility gate: required registry tasks missing: " + ", ".join(missing))
for task_id in sorted(required_ids):
    task = tasks[task_id]
    if task.get("milestone") != "M18" or task.get("required") is not True:
        raise SystemExit(f"compatibility gate: {task_id} must be required in M18")

matrix = root / "docs/compatibility-matrix.md"
if not matrix.is_file() or matrix.stat().st_size == 0:
    raise SystemExit("compatibility gate: docs/compatibility-matrix.md is missing or empty")
matrix_text = matrix.read_text(encoding="utf-8")
for heading in ("## Classification vocabulary", "## Current matrix", "## Support window", "## Evidence policy"):
    if heading not in matrix_text:
        raise SystemExit(f"compatibility gate: matrix lacks required section: {heading}")

classifications = {
    "Claimed supported", "Tested", "Partially tested", "Expected compatible",
    "Unsupported", "Unknown",
}
for classification in classifications:
    if f"| {classification} |" not in matrix_text:
        raise SystemExit(f"compatibility gate: matrix lacks classification vocabulary: {classification}")

current_matrix = matrix_text.split("## Current matrix", 1)[1].split("\n## ", 1)[0]
matrix_rows: list[tuple[str, str, str, str]] = []
for line in current_matrix.splitlines():
    if not line.startswith("|"):
        continue
    cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
    if len(cells) != 4 or cells[0] == "Yocto/Poky identity" or set(cells[0]) <= {"-", ":"}:
        continue
    identity, bitbake, classification, evidence = cells
    if classification not in classifications:
        raise SystemExit(f"compatibility gate: ambiguous matrix classification: {classification}")
    if not identity or not bitbake or not evidence:
        raise SystemExit("compatibility gate: matrix row lacks identity, BitBake, or evidence scope")
    if classification != "Unknown" and "compatibility" not in evidence.lower():
        raise SystemExit(
            f"compatibility gate: non-Unknown row lacks exact compatibility evidence link: {identity}"
        )
    matrix_rows.append((identity, bitbake, classification, evidence))
if not matrix_rows:
    raise SystemExit("compatibility gate: current matrix has no release rows")

claimed = [row for row in matrix_rows if row[2] == "Claimed supported"]
if claimed:
    for kind in ("latest", "older"):
        path = root / f"docs/compatibility-evidence/{kind}.toml"
        try:
            claim_evidence = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as exc:
            raise SystemExit(
                f"compatibility gate: claimed support lacks parseable live {kind} evidence: {exc}"
            )
        if claim_evidence.get("evidence_level") != "live" or claim_evidence.get("fixture_only") is not False:
            raise SystemExit(
                f"compatibility gate: claimed support cannot use fixture-only {kind} evidence"
            )

if mode == "--structure-only":
    print("compatibility milestone structure is valid")
    raise SystemExit(0)

incomplete = [task_id for task_id in sorted(required_ids) if tasks[task_id].get("status") != "DONE"]
if incomplete:
    details = ", ".join(f"{task_id}={tasks[task_id].get('status')}" for task_id in incomplete)
    raise SystemExit("compatibility gate: required tasks are incomplete: " + details)

parent_dependencies = set(tasks["COMPAT-001"].get("depends_on", []))
missing_parent_edges = sorted((required_ids - {"COMPAT-001"}) - parent_dependencies)
if missing_parent_edges:
    raise SystemExit("compatibility gate: COMPAT-001 lacks child dependencies: " + ", ".join(missing_parent_edges))

today = dt.date.today()
for kind in ("latest", "older"):
    path = root / f"docs/compatibility-evidence/{kind}.toml"
    try:
        evidence = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise SystemExit(f"compatibility gate: invalid live {kind} evidence: {exc}")
    required = (
        "schema_version", "observed_at", "expires_after_days", "official_source_url",
        "repository_url", "poky_commit", "yocto_release", "yocto_series",
        "bitbake_version", "yoctui_commit", "build_identity", "distro", "machine",
        "backend", "protocol_version", "commands", "capabilities", "workflows",
    )
    absent = [key for key in required if not evidence.get(key)]
    if absent:
        raise SystemExit(f"compatibility gate: {kind} evidence lacks exact fields: {', '.join(absent)}")
    if evidence.get("evidence_level") != "live" or evidence.get("fixture_only") is not False:
        raise SystemExit(f"compatibility gate: {kind} evidence is not an explicit non-fixture live run")
    try:
        observed = dt.date.fromisoformat(str(evidence["observed_at"]))
        expiry = int(evidence["expires_after_days"])
    except (TypeError, ValueError) as exc:
        raise SystemExit(f"compatibility gate: invalid {kind} evidence date policy: {exc}")
    if expiry < 1 or observed > today or (today - observed).days > expiry:
        raise SystemExit(f"compatibility gate: {kind} live evidence is stale or future-dated")

print("compatibility registry, documentation, and live evidence are current")
PY

if [[ "$mode" == "--structure-only" ]]; then
  exit 0
fi

for verifier in ./scripts/test-release-compatibility.sh ./scripts/verify-live-compatibility.sh; do
  if [[ ! -x "$verifier" ]]; then
    printf 'compatibility gate: missing executable verifier: %s\n' "$verifier" >&2
    exit 1
  fi
done

# Deterministic and network-free: model/catalog/probes, argv, dynamic UI, future release.
./scripts/test-release-compatibility.sh

# Evidence-only mode must never clone, fetch, or promote fixture evidence.
./scripts/verify-live-compatibility.sh --evidence-only

printf 'Yocto release capability compatibility verification passed\n'
