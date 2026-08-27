#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

# Keep every terminal layer explicit so a renamed or missing test cannot be
# hidden by a broad workspace invocation.
cargo test -q -p yoctui --bin yoctui ux_terminal_runtime
cargo test -q -p yoctui --test daemon_pty_runtime ux_terminal_real_pty
cargo test -q -p yoctui --test daemon_state_runtime reattach
cargo test -q -p yoctui --bin yoctui pty_attach
cargo test -q -p yoctui --bin yoctui daemon_raw::tests::raw_pty
cargo test -q -p yoctui-bitbake --lib pty_runner
cargo test -q -p yoctui-model --lib ux_terminal
cargo test -q -p yoctui-model --lib pty_session
cargo test -q -p yoctui-model --lib terminal_emulation
cargo test -q -p yoctui-app --lib ux_terminal
cargo test -q -p yoctui-app --lib mouse_runtime_routes_dialog_and_terminal
cargo test -q -p yoctui-ui --lib ux_terminal
cargo test -q -p yoctui-protocol --lib next_generation_pty
cargo test -q -p yoctui-e2e --lib next_generation_pty
cargo build -q -p yoctui

harness_root="$(mktemp -d /tmp/yoctui-workbench-terminal.XXXXXX)"
export XDG_CONFIG_HOME="$harness_root/config"
export XDG_STATE_HOME="$harness_root/state"
export XDG_RUNTIME_DIR="$harness_root/runtime"
mkdir -m 700 "$XDG_CONFIG_HOME" "$XDG_STATE_HOME" "$XDG_RUNTIME_DIR"

cleanup() {
  "$repo_root/target/debug/yoctui" daemon stop >/dev/null 2>&1 || true
  rm -rf -- "$harness_root"
}
trap cleanup EXIT

"$repo_root/target/debug/yoctui" daemon start >/dev/null

python3 - "$repo_root" <<'PY'
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

root = sys.argv[1]
ansi = re.compile(r"\x1b(?:\[[0-9;?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1b\\)|.)")


def collect(master, seconds=1):
    raw = bytearray()
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        ready, _, _ = select.select([master], [], [], .08)
        if not ready:
            continue
        try:
            raw.extend(os.read(master, 65536))
        except OSError:
            break
    return bytes(raw)


def semantic(raw):
    return ansi.sub("", raw.decode("utf-8", "replace"))


def send_and_expect(master, keys, expected, label):
    os.write(master, keys)
    raw = collect(master, 2)
    text = semantic(raw)
    if expected not in text:
        raise SystemExit(
            f"{label} did not render {expected!r} (returncode={process.poll()}): "
            f"text={text[-4000:]!r} raw={raw[-1000:]!r}"
        )
    if "�" in text:
        raise SystemExit(f"{label} emitted a replacement glyph")


config_dir = os.path.join(os.environ["XDG_CONFIG_HOME"], "yoctui")
os.mkdir(config_dir, mode=0o700)
with open(os.path.join(config_dir, "config.toml"), "w", encoding="utf-8") as config:
    config.write("")
with open(os.path.join(config_dir, "session.toml"), "w", encoding="utf-8") as session:
    session.write(
        '[onboarding]\n'
        'schema_version = 1\n'
        'current = "environment"\n'
        'completed = []\n'
        'skipped = []\n'
        'dismissed = true\n'
    )

master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 160, 0, 0))


def become_session_leader():
    os.setsid()
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)


process = subprocess.Popen(
    [os.path.join(root, "target/debug/yoctui"), "--no-color"],
    stdin=slave,
    stdout=slave,
    stderr=slave,
    env=os.environ.copy(),
    preexec_fn=become_session_leader,
)
os.close(slave)
startup = bytearray()
deadline = time.monotonic() + 8
while time.monotonic() < deadline and (
    b"\x1b[?1049h" not in startup
    or b"yoctui" not in startup.lower()
    or b"connected" not in startup.lower()
):
    startup.extend(collect(master, .2))
if (
    b"\x1b[?1049h" not in startup
    or b"yoctui" not in startup.lower()
    or b"connected" not in startup.lower()
):
    raise SystemExit(
        "terminal workbench did not attach to the isolated daemon: "
        + semantic(startup)[-4000:]
    )

send_and_expect(master, b"\x02t", "Terminal Sessions", "terminal prefix route")
send_and_expect(
    master,
    b"\x02%",
    "split requested",
    "terminal split route",
)
send_and_expect(master, b"\x02?", "Help opened", "terminal prefix help")
os.write(master, b"\x1b")
collect(master, .3)

fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
narrow = semantic(collect(master, 1))
if "Terminal Sessions" not in narrow or "�" in narrow:
    raise SystemExit(f"terminal workbench resize lost identity: {narrow[-4000:]}")

os.write(master, b"q")
try:
    process.wait(timeout=3)
except subprocess.TimeoutExpired:
    process.kill()
    process.wait(timeout=2)
shutdown = collect(master, .2)
os.close(master)
if process.returncode != 0 or b"\x1b[?1049l" not in shutdown:
    raise SystemExit(f"terminal workbench PTY exited with {process.returncode}")

print("workbench terminal lifecycle and controlling-PTY matrix passed")
PY
