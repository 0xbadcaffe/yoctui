#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mode="${1:---evidence-only}"
role="${2:-}"
case "$mode" in
  --evidence-only)
    [[ -z "$role" ]] || { printf 'usage: %s [--evidence-only|--live {latest|older|development}]\n' "$0" >&2; exit 2; }
    ;;
  --live)
    case "$role" in latest|older|development) ;; *) printf 'usage: %s [--evidence-only|--live {latest|older|development}]\n' "$0" >&2; exit 2 ;; esac
    ;;
  *) printf 'usage: %s [--evidence-only|--live {latest|older|development}]\n' "$0" >&2; exit 2 ;;
esac

./scripts/verify-live-compatibility.sh --evidence-only

python3 <<'PY'
from __future__ import annotations

import datetime as dt
from pathlib import Path
import re
import tomllib

root = Path.cwd()
records = {
    kind: tomllib.loads((root / "docs" / "compatibility-evidence" / f"{kind}.toml").read_text())
    for kind in ("latest", "older")
}
latest = records["latest"]
older = records["older"]

def version(value: str) -> tuple[int, ...]:
    if not re.fullmatch(r"[0-9]+(?:\.[0-9]+)+", value):
        raise SystemExit(f"compatibility matrix: non-numeric correlated version: {value}")
    return tuple(int(part) for part in value.split("."))

if version(older["yocto_release"]) >= version(latest["yocto_release"]):
    raise SystemExit("compatibility matrix: older Yocto release is not older than latest")
if version(older["bitbake_version"]) >= version(latest["bitbake_version"]):
    raise SystemExit("compatibility matrix: older BitBake is not materially older than latest")
for field in ("yocto_series", "poky_commit", "oe_core_commit", "bitbake_commit", "meta_yocto_commit", "build_identity"):
    if older[field] == latest[field]:
        raise SystemExit(f"compatibility matrix: live anchors share unexpected {field}")
if older["source_composition"] != "poky_checkout" or latest["source_composition"] != "split_components":
    raise SystemExit("compatibility matrix: exact official source compositions are not distinguished")

today = dt.date.today()
for kind, record in records.items():
    support_year, support_month = map(int, record["support_until"].split("-"))
    support_end = dt.date(support_year, support_month, 1)
    if support_end < today.replace(day=1):
        raise SystemExit(f"compatibility matrix: {kind} support policy has expired")

matrix = (root / "docs" / "compatibility-matrix.md").read_text(encoding="utf-8")
for required in (
    "compatibility-evidence/latest.toml",
    "compatibility-evidence/older.toml",
    "Partially tested",
    "Future/development and mixed identities: **Unknown**",
):
    if required not in matrix:
        raise SystemExit(f"compatibility matrix: policy document lacks {required!r}")
matrix_lower = matrix.lower()
for series in (latest["yocto_series"], older["yocto_series"]):
    if series.lower() not in matrix_lower:
        raise SystemExit(f"compatibility matrix: policy document lacks series {series!r}")

print(
    "compatibility matrix evidence passed: "
    f"older={older['yocto_release']}/{older['bitbake_version']} "
    f"latest={latest['yocto_release']}/{latest['bitbake_version']}; "
    "development remains optional/non-claiming"
)
PY

if [[ "$mode" == "--evidence-only" ]]; then
  exit 0
fi

if [[ "${YOCTUI_LIVE_COMPATIBILITY:-0}" != 1 ]]; then
  printf '%s\n' 'live compatibility matrix is opt-in; set YOCTUI_LIVE_COMPATIBILITY=1' >&2
  exit 2
fi

matrix_root="$(mktemp -d "$repo_root/.yoctui-live-compat-${role}.XXXXXX")"
runtime_dir="$matrix_root/runtime"
state_dir="$matrix_root/state"
source_dir="$matrix_root/source"
build_dir="$matrix_root/build"
artifact_dir="${YOCTUI_COMPAT_ARTIFACT_DIR:-$repo_root/artifacts/compatibility/$role}"
mkdir -p "$runtime_dir" "$state_dir" "$source_dir" "$artifact_dir"
chmod 700 "$runtime_dir" "$state_dir"

cleanup() {
  YOCTUI_BUILD_DIR="$build_dir" XDG_RUNTIME_DIR="$runtime_dir" XDG_STATE_HOME="$state_dir" \
    "$repo_root/target/debug/yoctui" daemon stop >/dev/null 2>&1 || true
  rm -rf -- "$matrix_root"
}
trap cleanup EXIT

fetch_exact() {
  local repository="$1"
  local revision="$2"
  local destination="$3"
  git init -q "$destination"
  git -C "$destination" remote add origin "$repository"
  git -C "$destination" fetch -q --depth 1 origin "$revision"
  git -C "$destination" checkout -q --detach FETCH_HEAD
  [[ "$(git -C "$destination" rev-parse HEAD)" == "$revision" ]] || {
    printf 'live compatibility matrix: checkout mismatch for %s\n' "$destination" >&2
    exit 1
  }
}

if [[ "$role" == "development" ]]; then
  development_revision="${YOCTUI_COMPAT_DEVELOPMENT_REVISION:-}"
  [[ "$development_revision" =~ ^[0-9a-f]{40}$ ]] || {
    printf '%s\n' 'development live role requires exact YOCTUI_COMPAT_DEVELOPMENT_REVISION' >&2
    exit 2
  }
  development_repository="${YOCTUI_COMPAT_DEVELOPMENT_REPOSITORY:-https://git.yoctoproject.org/poky}"
  fetch_exact "$development_repository" "$development_revision" "$source_dir/poky"
  init_script="$source_dir/poky/oe-init-build-env"
  expected_release=""
  expected_bitbake=""
  expected_machine="qemux86-64"
  printf '%s\n' 'development role is optional diagnostic evidence and cannot create a support claim'
else
  mapfile -t record < <(python3 - "$role" <<'PY'
import pathlib, sys, tomllib
d = tomllib.loads((pathlib.Path("docs/compatibility-evidence") / f"{sys.argv[1]}.toml").read_text())
for key in (
    "source_composition", "repository_url", "poky_commit",
    "oe_core_repository_url", "oe_core_commit", "bitbake_repository_url",
    "bitbake_commit", "meta_yocto_repository_url", "meta_yocto_commit",
    "yocto_release", "bitbake_version", "machine",
):
    print(d[key])
PY
  )
  composition="${record[0]}"
  if [[ "$composition" == "poky_checkout" ]]; then
    fetch_exact "${record[1]}" "${record[2]}" "$source_dir/poky"
    init_script="$source_dir/poky/oe-init-build-env"
  else
    fetch_exact "${record[3]}" "${record[4]}" "$source_dir/openembedded-core"
    fetch_exact "${record[5]}" "${record[6]}" "$source_dir/bitbake"
    fetch_exact "${record[7]}" "${record[8]}" "$source_dir/meta-yocto"
    init_script="$source_dir/openembedded-core/oe-init-build-env"
  fi
  expected_release="${record[9]}"
  expected_bitbake="${record[10]}"
  expected_machine="${record[11]}"
fi

[[ -x "$init_script" ]] || { printf 'live compatibility matrix: init script missing: %s\n' "$init_script" >&2; exit 1; }
unset PYENV_DIR PYENV_HOOK_PATH PYENV_VERSION
export PATH="/usr/bin:/bin:$PATH"
set +u
source "$init_script" "$build_dir" >/dev/null
set -u

if [[ "${composition:-poky_checkout}" == "split_components" ]]; then
  bitbake-layers add-layer "$source_dir/meta-yocto/meta-poky"
  bitbake-layers add-layer "$source_dir/meta-yocto/meta-yocto-bsp"
  # OE-Core's standalone initializer defaults to nodistro.  The release
  # evidence is explicitly Poky, so select the checked-out meta-poky distro in
  # this isolated build rather than inventing identity from the layer names.
  printf '\nDISTRO = "poky"\n' >>"$build_dir/conf/local.conf"
fi

if [[ ! -x "$repo_root/target/debug/yoctui" ]]; then
  cargo build --locked -p yoctui
fi
export YOCTUI_BUILD_DIR="$build_dir"
export XDG_RUNTIME_DIR="$runtime_dir"
export XDG_STATE_HOME="$state_dir"
export YOCTUI_DAEMON_LOG="$artifact_dir/daemon.log"

"$repo_root/target/debug/yoctui" daemon start
"$repo_root/target/debug/yoctui" --build-dir "$build_dir" doctor --json >"$artifact_dir/doctor.json"
"$repo_root/target/debug/yoctui" --build-dir "$build_dir" inspect >"$artifact_dir/inspect.txt"
"$repo_root/target/debug/yoctui" --build-dir "$build_dir" recipes | wc -l >"$artifact_dir/recipe-count.txt"
"$repo_root/target/debug/yoctui" --build-dir "$build_dir" layers | wc -l >"$artifact_dir/layer-count.txt"
"$repo_root/target/debug/yoctui" --build-dir "$build_dir" config MACHINE >"$artifact_dir/machine.txt"

python3 - "$artifact_dir/doctor.json" "$expected_release" "$expected_bitbake" "$expected_machine" <<'PY'
import json, pathlib, sys
d = json.loads(pathlib.Path(sys.argv[1]).read_text())
expected_release, expected_bitbake, expected_machine = sys.argv[2:]
environment = d["environment"]
actual_release = environment.get("poky", {}).get("value", {}).get("version")
actual_bitbake = environment.get("bitbake_version", {}).get("value")
actual_machine = environment.get("machine", {}).get("value")
if expected_release and actual_release != expected_release:
    raise SystemExit(f"live compatibility matrix: release mismatch {actual_release!r}")
if expected_bitbake and actual_bitbake != expected_bitbake:
    raise SystemExit(f"live compatibility matrix: BitBake mismatch {actual_bitbake!r}")
if actual_machine != expected_machine:
    raise SystemExit(f"live compatibility matrix: machine mismatch {actual_machine!r}")
if d.get("authority") != "current" or not d.get("capabilities"):
    raise SystemExit("live compatibility matrix: daemon capability authority is absent")
PY

YOCTUI_LIVE_BITBAKE=1 \
YOCTUI_LIVE_BUILD_DIR="$build_dir" \
YOCTUI_OE_INIT_BUILD_ENV="$init_script" \
YOCTUI_LIVE_TIMEOUT="${YOCTUI_LIVE_TIMEOUT:-300}" \
  "$repo_root/scripts/verify-live-bitbake.sh" | tee "$artifact_dir/bitbake-smoke.txt"

{
  printf 'role=%s\n' "$role"
  printf 'observed_at=%s\n' "$(date -u +%F)"
  printf 'bitbake=%s\n' "$(bitbake --version)"
  printf 'build_identity='; sha256sum "$build_dir/conf/local.conf" "$build_dir/conf/bblayers.conf" | sha256sum | cut -d' ' -f1
} >"$artifact_dir/manifest.txt"

printf 'live compatibility matrix passed: role=%s artifacts=%s\n' "$role" "$artifact_dir"
