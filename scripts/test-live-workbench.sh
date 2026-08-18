#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
build_dir="${1:-${YOCTUI_LIVE_BUILD_DIR:-}}"
if [[ -z "$build_dir" ]]; then
  printf '%s\n' 'usage: test-live-workbench.sh /absolute/path/to/poky/build' >&2
  exit 2
fi
build_dir="$(readlink -f "$build_dir")"
if [[ ! -f "$build_dir/conf/bblayers.conf" ]]; then
  printf 'live workbench: missing %s/conf/bblayers.conf\n' "$build_dir" >&2
  exit 2
fi

source_poky="${YOCTUI_POKY_SOURCE:-$(dirname "$build_dir")}"
if [[ ! -f "$source_poky/oe-init-build-env" ]]; then
  printf 'live workbench: missing %s/oe-init-build-env\n' "$source_poky" >&2
  exit 2
fi

work_root="$(mktemp -d /tmp/yoctui-live-workbench.XXXXXX)"
trap 'rm -rf "$work_root"' EXIT
export XDG_CONFIG_HOME="$work_root/config"
export XDG_STATE_HOME="$work_root/state"
export XDG_RUNTIME_DIR="$work_root/runtime"
mkdir -m 700 "$XDG_CONFIG_HOME" "$XDG_STATE_HOME" "$XDG_RUNTIME_DIR"
unset YOCTUI_BACKEND
unset NO_COLOR

set +u
source "$source_poky/oe-init-build-env" "$build_dir" >/dev/null
set -u
cd "$repo_root"

cargo build -p yoctui >/dev/null
binary="$repo_root/target/debug/yoctui"
artifact_dir="${YOCTUI_LIVE_ARTIFACT_DIR:-$repo_root/artifacts/release-quality/live-workbench}"
mkdir -p "$artifact_dir"

"$binary" --backend bridge --build-dir "$build_dir" inspect >"$work_root/inspect.txt"
grep -Fq 'MACHINE=qemux86-64' "$work_root/inspect.txt"
grep -Fq 'DISTRO=poky' "$work_root/inspect.txt"
grep -Fq 'Yocto/OpenEmbedded release:' "$work_root/inspect.txt"

"$binary" --backend bridge --build-dir "$build_dir" layers >"$work_root/layers.txt"
grep -Eq '^core[[:space:]]' "$work_root/layers.txt"
grep -Eq '^yocto[[:space:]]' "$work_root/layers.txt"
grep -Eq '^yoctobsp[[:space:]]' "$work_root/layers.txt"

"$binary" --backend bridge --build-dir "$build_dir" recipes >"$work_root/recipes.txt"
grep -Eq '^core-image-minimal[[:space:]]' "$work_root/recipes.txt"
grep -Eq '^busybox[[:space:]]' "$work_root/recipes.txt"
recipe_count="$(wc -l <"$work_root/recipes.txt")"
if (( recipe_count < 100 )); then
  printf 'live workbench: unexpectedly small recipe inventory: %s\n' "$recipe_count" >&2
  exit 1
fi

python3 - "$binary" "$build_dir" "$artifact_dir/pty-semantic.txt" <<'PY'
import fcntl
import os
import pty
import re
import select
import struct
import subprocess
import sys
import termios
import time

binary, build_dir, artifact = sys.argv[1:]
master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 48, 160, 0, 0))
process = subprocess.Popen(
    [binary, "--backend", "bridge", "--build-dir", build_dir],
    stdin=slave,
    stdout=slave,
    stderr=slave,
    env=os.environ.copy(),
    start_new_session=True,
)
os.close(slave)
raw = bytearray()
deadline = time.monotonic() + 45
while time.monotonic() < deadline:
    ready, _, _ = select.select([master], [], [], 0.25)
    if ready:
        try:
            raw.extend(os.read(master, 65536))
        except OSError:
            break
    if b"Focus Workspace" in raw and b"qemux86-64" in raw and b"OVERVIEW" in raw:
        break

os.write(master, b"q")
try:
    process.wait(timeout=5)
except subprocess.TimeoutExpired:
    process.kill()
    process.wait(timeout=2)
os.close(master)

payload = bytes(raw)
sgr_codes = set(re.findall(rb"\x1b\[[0-9;]*m", payload))
color_codes = {
    code
    for code in sgr_codes
    if re.search(rb"(?:\[|;)(?:3[0-8]|4[0-8]|9[0-7]|10[0-7])(?:;|m)", code)
}
if len(color_codes) < 2:
    rendered_codes = ", ".join(repr(code) for code in sorted(sgr_codes)[:20])
    raise SystemExit(
        "live workbench: distinct colored terminal sequences were not emitted; "
        f"observed {rendered_codes}"
    )

text = payload.decode("utf-8", "replace")
normalized = re.sub(r"\x1b\[[0-9;?]*[ -/]*[@-~]", "", text)
normalized = "\n".join(line.rstrip() for line in normalized.splitlines() if line.strip())
for anchor in (
    "yoctui",
    "qemux86-64",
    "poky",
    "OVERVIEW",
    "CONTENT",
    "BUILD",
    "Focus Workspace",
    "Tab Inspector",
    "Shift+Tab Navigator",
):
    if anchor not in normalized:
        raise SystemExit(f"live workbench: missing PTY anchor: {anchor}")
if "Daemon unavailable; interactive runtime is local" in normalized:
    raise SystemExit("live workbench: daemon fallback notice obscured the workbench")
if process.returncode != 0:
    raise SystemExit(f"live workbench: TUI exited with {process.returncode}")

with open(artifact, "w", encoding="utf-8") as output:
    output.write(normalized[-65536:])
PY

cp "$work_root/inspect.txt" "$artifact_dir/inspect.txt"
cp "$work_root/layers.txt" "$artifact_dir/layers.txt"
{
  printf 'build_dir=%s\n' "$build_dir"
  printf 'recipe_count=%s\n' "$recipe_count"
  printf '%s\n' 'recipes=core-image-minimal,busybox'
  printf '%s\n' 'pty=colored-wide-workbench-passed'
} >"$artifact_dir/summary.txt"

printf 'live workbench: metadata and colored PTY passed (%s recipes)\n' "$recipe_count"
