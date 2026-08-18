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

set +u
source "$source_poky/oe-init-build-env" "$build_dir" >/dev/null
set -u
cd "$repo_root"

binary="${YOCTUI_LIVE_BINARY:-}"
if [[ -z "$binary" ]]; then
  cargo build -p yoctui >/dev/null
  binary="$repo_root/target/debug/yoctui"
else
  binary="$(readlink -f "$binary")"
  if [[ ! -x "$binary" ]]; then
    printf 'live workbench: binary is not executable: %s\n' "$binary" >&2
    exit 2
  fi
fi
artifact_dir="${YOCTUI_LIVE_ARTIFACT_DIR:-$work_root/artifacts}"
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
import signal
import struct
import subprocess
import sys
import termios
import time

binary, build_dir, artifact = sys.argv[1:]
master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 48, 160, 0, 0))


def become_session_leader():
    os.setsid()
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)


process = subprocess.Popen(
    [binary, "--backend", "bridge", "--build-dir", build_dir],
    stdin=slave,
    stdout=slave,
    stderr=slave,
    env=os.environ.copy(),
    preexec_fn=become_session_leader,
)
os.close(slave)
raw = bytearray()


def has_rendered_anchor(rendered, anchor):
    return "".join(anchor.split()) in "".join(rendered.split())


def collect_until(anchors, timeout):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        ready, _, _ = select.select([master], [], [], 0.25)
        if ready:
            try:
                raw.extend(os.read(master, 65536))
            except OSError:
                break
        visible = re.sub(
            r"\x1b\[[0-9;?]*[ -/]*[@-~]", "", raw.decode("utf-8", "replace")
        )
        if all(
            has_rendered_anchor(visible, anchor.decode("utf-8", "replace"))
            for anchor in anchors
        ):
            return
    missing = [
        anchor.decode("utf-8", "replace")
        for anchor in anchors
        if not has_rendered_anchor(visible, anchor.decode("utf-8", "replace"))
    ]
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait(timeout=2)
    raise SystemExit(
        f"live workbench: timed out waiting for PTY anchors: {missing}\n"
        f"last rendered output:\n{visible[-4000:]}"
    )


def collect_for(duration):
    deadline = time.monotonic() + duration
    while time.monotonic() < deadline:
        ready, _, _ = select.select([master], [], [], 0.1)
        if ready:
            try:
                raw.extend(os.read(master, 65536))
            except OSError:
                return


collect_until((b"Focus ", b"qemux86-64", b"OVERVIEW"), 45)
os.write(master, b"\x10")
collect_for(0.5)
os.write(master, b"Choose theme\r")
collect_for(0.5)
os.write(master, b"\x1b[B\r")
collect_for(0.5)

os.write(master, b"q")
try:
    process.wait(timeout=5)
except subprocess.TimeoutExpired:
    os.killpg(process.pid, signal.SIGKILL)
    process.wait(timeout=2)
os.close(master)

payload = bytes(raw)
alternate_screen = payload.find(b"\x1b[?1049h")
if alternate_screen < 0:
    raise SystemExit("live workbench: terminal never entered the alternate screen")
for marker in (b"NOTE:", b"WARNING:", b"Traceback (most recent call last)"):
    if marker in payload:
        raise SystemExit(
            "live workbench: backend diagnostics leaked into the terminal: "
            + marker.decode("ascii")
        )

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
):
    if not has_rendered_anchor(normalized, anchor):
        raise SystemExit(f"live workbench: missing PTY anchor: {anchor}")
focus_routes = (
    ("Focus Navigator", "Tab Workspace", "Shift+Tab Inspector"),
    ("Focus Workspace", "Tab Inspector", "Shift+Tab Navigator"),
    ("Focus Inspector", "Tab Navigator", "Shift+Tab Workspace"),
)
if not any(
    all(has_rendered_anchor(normalized, anchor) for anchor in route)
    for route in focus_routes
):
    raise SystemExit("live workbench: missing explicit pane-focus route")
if "Daemon unavailable; interactive runtime is local" in normalized:
    raise SystemExit("live workbench: daemon fallback notice obscured the workbench")
if process.returncode != 0:
    raise SystemExit(f"live workbench: TUI exited with {process.returncode}")

session_path = os.path.join(os.environ["XDG_CONFIG_HOME"], "yoctui", "session.toml")
try:
    with open(session_path, encoding="utf-8") as session_file:
        session = session_file.read()
except OSError as error:
    raise SystemExit(f"live workbench: theme session was not persisted: {error}") from error
if 'theme = "white-classic"' not in session:
    raise SystemExit(f"live workbench: WhiteClassic theme was not persisted: {session!r}")

with open(artifact, "w", encoding="utf-8") as output:
    output.write(normalized[-65536:])
PY

cp "$work_root/inspect.txt" "$artifact_dir/inspect.txt"
cp "$work_root/layers.txt" "$artifact_dir/layers.txt"
{
  printf 'build_dir=%s\n' "$build_dir"
  printf 'recipe_count=%s\n' "$recipe_count"
  printf '%s\n' 'recipes=core-image-minimal,busybox'
  printf '%s\n' 'pty=clean-colored-wide-workbench-and-theme-passed'
} >"$artifact_dir/summary.txt"

printf 'live workbench: metadata, clean colored PTY, and theme passed (%s recipes)\n' "$recipe_count"
