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
matrix = root / "docs/supported-releases.md"
if not matrix.is_file() or matrix.stat().st_size == 0:
    raise SystemExit("compatibility gate: docs/supported-releases.md is missing or empty")
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

compatibility = root / "docs/compatibility.md"
readme = root / "README.md"
manual = root / "docs/user-guide.md"
for path in (compatibility, manual, readme):
    if not path.is_file() or path.stat().st_size == 0:
        raise SystemExit(f"compatibility gate: {path.relative_to(root)} is missing or empty")
if "Yoctui functionality is Yocto-feature-correlated" not in compatibility.read_text(encoding="utf-8"):
    raise SystemExit("compatibility gate: docs/compatibility.md lacks the product compatibility rule")
if "docs/user-guide.md" not in readme.read_text(encoding="utf-8") or "compatibility.md" not in manual.read_text(encoding="utf-8"):
    raise SystemExit("compatibility gate: README/manual must expose the compatibility guide")

if mode == "--structure-only":
    print("compatibility structure is valid")
    raise SystemExit(0)

today = dt.date.today()
evidence_by_kind: dict[str, dict] = {}
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
    evidence_by_kind[kind] = evidence

claimed = [row for row in matrix_rows if row[2] == "Claimed supported"]
if len(claimed) < 2:
    raise SystemExit(
        "compatibility gate: live validation requires claimed-supported latest and older rows"
    )
for kind, evidence in evidence_by_kind.items():
    release = str(evidence["yocto_release"])
    bitbake = str(evidence["bitbake_version"])
    link = f"compatibility-evidence/{kind}.toml"
    matches = [
        row for row in claimed
        if release in row[0] and row[1].strip("`") == bitbake and link in row[3]
    ]
    if len(matches) != 1:
        raise SystemExit(
            f"compatibility gate: {kind} live identity is not exactly represented as Claimed supported"
        )

print("compatibility contracts, documentation, and live evidence are current")
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
