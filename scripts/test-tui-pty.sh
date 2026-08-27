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
    isolated_env = os.environ.copy()
    for variable, directory in (
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_RUNTIME_DIR", "runtime"),
    ):
        path = os.path.join(build, directory)
        os.mkdir(path, mode=0o700)
        isolated_env[variable] = path
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    def become_session_leader():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)
    proc = subprocess.Popen([os.path.join(root, "target/debug/yoctui"), "--backend", "process", "--no-color"], stdin=slave, stdout=slave, stderr=slave, preexec_fn=become_session_leader, env=isolated_env)
    os.close(slave)
    raw = bytearray()
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        ready, _, _ = select.select([master], [], [], .2)
        if ready:
            try: raw.extend(os.read(master, 65536))
            except OSError: break
            if b"yoctui" in raw.lower(): break
    os.write(master, b"\x1b")
    time.sleep(.2)
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
    if b"yoctui" not in raw.lower() or proc.returncode != 0:
        stamp = str(int(time.time()))
        open(os.path.join(artifact_root, f"pty-{stamp}.ansi"), "wb").write(raw[-262144:])
        open(os.path.join(artifact_root, f"pty-{stamp}.log"), "w").write(text[-32768:])
        raise SystemExit(f"PTY acceptance failed: returncode={proc.returncode}")
    assert "\x1b[?1049l" in text, "alternate screen was not restored"
print("real PTY TUI harness passed")
PY
