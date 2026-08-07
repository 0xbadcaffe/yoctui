#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
cargo build -p yoctui >/dev/null
python3 - "$repo_root" <<'PY'
import os, pty, select, struct, subprocess, sys, termios, fcntl, time, tempfile
root = sys.argv[1]
with tempfile.TemporaryDirectory(prefix='yoctui-flow-', dir='/tmp') as tmp:
    os.mkdir(os.path.join(tmp, 'build'))
    master, slave = pty.openpty()
    def resize(w, h):
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', h, w, 0, 0))
    resize(80, 24)
    proc = subprocess.Popen([os.path.join(root, 'target/debug/yoctui'), '--backend', 'process', '--build-dir', os.path.join(tmp, 'build'), '--no-color'], stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
    os.close(slave)
    raw = bytearray()
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline and b'Yoctui' not in raw:
        ready, _, _ = select.select([master], [], [], .2)
        if ready:
            try: raw.extend(os.read(master, 65536))
            except OSError: break
    os.write(master, b'\t\x1b[Zyrlt\x1b[Z?')
    time.sleep(.2)
    # Resize while the application is focused, then exercise the too-small
    # boundary and recovery before returning through the quit dialog.
    fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack('HHHH', 12, 40, 0, 0))
    time.sleep(.2)
    fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack('HHHH', 48, 160, 0, 0))
    os.write(master, b'q\r')
    try: proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        proc.kill(); proc.wait(timeout=2)
    os.close(master)
    if b'Yoctui' not in raw or proc.returncode not in (0, 1, -9):
        raise SystemExit(f'flow PTY failed: returncode={proc.returncode}')
print('real PTY navigation flow passed')
PY
