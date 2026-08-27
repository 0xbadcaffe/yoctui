#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

cargo test -q -p yoctui-e2e next_generation_keymap
cargo test -q -p yoctui-model ux_keymap
cargo test -q -p yoctui-app ux_focus
cargo test -q -p yoctui-app ux_scroll
cargo build -q -p yoctui

python3 - "$repo_root" <<'PY'
import fcntl
import os
import pty
import re
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time

root = sys.argv[1]
ansi = re.compile(r"\x1b(?:\[[0-9;?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1b\\)|.)")

def collect(master, seconds=2):
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
    raw = collect(master)
    text = semantic(raw)
    if expected not in text:
        raise SystemExit(
            f"{label} did not render {expected!r} (returncode={proc.poll()}): "
            f"text={text[-4000:]!r} raw={raw[-1000:]!r} "
            f"startup_tail={semantic(startup)[-4000:]!r}"
        )
    if "�" in text:
        raise SystemExit(f"{label} emitted a replacement glyph")
    return raw

with tempfile.TemporaryDirectory(prefix="yoctui-workbench-keymap-", dir="/tmp") as tmp:
    env = os.environ.copy()
    for variable, directory in (
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_RUNTIME_DIR", "runtime"),
    ):
        path = os.path.join(tmp, directory)
        os.mkdir(path, mode=0o700)
        env[variable] = path
    yoctui_config = os.path.join(env["XDG_CONFIG_HOME"], "yoctui")
    os.mkdir(yoctui_config, mode=0o700)
    with open(os.path.join(yoctui_config, "config.toml"), "w", encoding="utf-8") as config:
        config.write("")
    with open(os.path.join(yoctui_config, "session.toml"), "w", encoding="utf-8") as session:
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
    binary = os.path.join(root, "target/debug/yoctui")

    def become_session_leader():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    proc = subprocess.Popen(
        [binary, "--backend", "process", "--no-color"],
        stdin=slave,
        stdout=slave,
        stderr=slave,
        preexec_fn=become_session_leader,
        env=env,
    )
    os.close(slave)
    startup = bytearray()
    startup_deadline = time.monotonic() + 8
    while time.monotonic() < startup_deadline and (
        b"\x1b[?1049h" not in startup or b"yoctui" not in startup.lower()
    ):
        startup.extend(collect(master, .2))
    if b"yoctui" not in startup.lower() or b"\x1b[?1049h" not in startup:
        diagnostic = startup.decode("utf-8", "replace")[-4000:]
        raise SystemExit(
            "workbench did not enter its alternate-screen shell "
            f"(returncode={proc.poll()}): {diagnostic}"
        )

    # The isolated session has onboarding dismissed so every route is deterministic.
    send_and_expect(master, b"\x1b[21~", "Application menu", "F10 application menu")
    os.write(master, b"\x1b")
    collect(master)
    send_and_expect(master, b"\x10", "Command Palette", "Ctrl+P command palette")
    os.write(master, b"\x1b")
    collect(master)
    send_and_expect(master, b"\x1bOQ", "Tasks", "F2 Tasks route")
    send_and_expect(master, b"a", "Tasks actions", "context menu route")
    os.write(master, b"\x1b")
    collect(master)

    # Real terminal scrolling/chords must remain accepted and bounded.
    os.write(master, b"\x1b[6~\x1b[5~\x1b[H\x1b[FggG/compile\r\x15")
    scrolled = semantic(collect(master, 1))
    if "�" in scrolled:
        raise SystemExit("scroll/search key matrix emitted a replacement glyph")

    # Resize across narrow and wide topology without changing the active screen.
    fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    narrow = semantic(collect(master, 1))
    if "Panes:" not in narrow or "Tasks" not in narrow:
        raise SystemExit(f"narrow resize lost pane/screen identity: {narrow[-4000:]}")
    fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 160, 0, 0))
    wide = semantic(collect(master, 1))
    if "Tasks" not in wide:
        raise SystemExit(f"wide resize lost Tasks identity: {wide[-4000:]}")

    # SGR right-click uses the same context-menu authority as keyboard `a`.
    send_and_expect(master, b"\x1b[<2;60;12M", "Tasks actions", "right-click context menu")
    os.write(master, b"\x1b\x1bq\r")
    try:
        proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=2)
    shutdown = collect(master, .2)
    os.close(master)
    if proc.returncode != 0 or b"\x1b[?1049l" not in shutdown:
        raise SystemExit(f"workbench keymap PTY exited with {proc.returncode}")

print("workbench real-PTY keymap/menu/focus/scroll matrix passed")
PY
