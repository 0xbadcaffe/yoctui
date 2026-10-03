#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
cargo build -p yoctui >/dev/null
python3 - "$repo_root" <<'PY'
import os, pty, select, struct, subprocess, sys, tempfile, time, termios, fcntl

root = sys.argv[1]
sys.path.insert(0, os.path.join(root, "scripts"))
from pty_acceptance import binary_path, isolated_environment, TerminalAcceptance
artifact_root = os.path.join(root, "artifacts", "release-quality")
os.makedirs(artifact_root, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="yoctui-pty-", dir="/tmp") as build:
    isolated_env = isolated_environment(build)
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    def become_session_leader():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)
    proc = subprocess.Popen([binary_path(root), "--backend", "process", "--no-color"], stdin=slave, stdout=slave, stderr=slave, preexec_fn=become_session_leader, env=isolated_env)
    os.close(slave)
    raw = bytearray()
    terminal = TerminalAcceptance(master, proc, raw, 80, 24)
    try:
        terminal.dismiss_onboarding()
        terminal.finish()
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait(timeout=2)
    os.close(master)
    text = bytes(raw).decode("utf-8", "replace")
    if b"yoctui" not in raw.lower() or proc.returncode != 0:
        stamp = str(int(time.time()))
        open(os.path.join(artifact_root, f"pty-{stamp}.ansi"), "wb").write(raw[-262144:])
        open(os.path.join(artifact_root, f"pty-{stamp}.log"), "w").write(text[-32768:])
        raise SystemExit(f"PTY acceptance failed: returncode={proc.returncode}")
    assert "\x1b[?1049l" in text, "alternate screen was not restored"
print("real PTY TUI harness passed")
PY
