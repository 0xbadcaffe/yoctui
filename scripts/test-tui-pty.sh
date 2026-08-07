#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
cargo build -p yoctui >/dev/null
python3 - "$repo_root" <<'PY'
import os, pty, select, struct, subprocess, sys, tempfile, time, termios, fcntl

root = sys.argv[1]
artifact_root = os.path.join(root, "artifacts", "release-quality")
os.makedirs(artifact_root, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="yoctui-pty-", dir="/tmp") as build:
    work = os.path.join(build, "build")
    os.mkdir(work)
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 80, 24, 0, 0))
    proc = subprocess.Popen([os.path.join(root, "target/debug/yoctui"), "--backend", "process", "--build-dir", work, "--no-color"], stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
    os.close(slave)
    raw = bytearray()
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        ready, _, _ = select.select([master], [], [], .2)
        if ready:
            try: raw.extend(os.read(master, 65536))
            except OSError: break
            if b"Yoctui" in raw: break
    os.write(master, b"q")
    try: proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=2)
    while True:
        ready, _, _ = select.select([master], [], [], .1)
        if not ready: break
        try: raw.extend(os.read(master, 65536))
        except OSError: break
    os.close(master)
    text = bytes(raw).decode("utf-8", "replace")
    if b"Yoctui" not in raw or proc.returncode not in (0, 1):
        stamp = str(int(time.time()))
        open(os.path.join(artifact_root, f"pty-{stamp}.ansi"), "wb").write(raw[-262144:])
        open(os.path.join(artifact_root, f"pty-{stamp}.log"), "w").write(text[-32768:])
        raise SystemExit(f"PTY acceptance failed: returncode={proc.returncode}")
    assert "\x1b[?1049l" in text, "alternate screen was not restored"
print("real PTY TUI harness passed")
PY
