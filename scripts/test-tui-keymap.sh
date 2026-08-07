#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
cargo build -p yoctui >/dev/null
python3 - "$repo_root" <<'PY'
import os, pty, select, struct, subprocess, sys, termios, fcntl, time
root = sys.argv[1]
with __import__('tempfile').TemporaryDirectory(prefix='yoctui-keymap-', dir='/tmp') as tmp:
    os.mkdir(os.path.join(tmp, 'build'))
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 100, 30, 0, 0))
    proc = subprocess.Popen([os.path.join(root, 'target/debug/yoctui'), '--backend', 'process', '--build-dir', os.path.join(tmp, 'build'), '--no-color'], stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
    os.close(slave)
    raw = bytearray()
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline and b'Yoctui' not in raw:
        ready, _, _ = select.select([master], [], [], .2)
        if ready:
            try: raw.extend(os.read(master, 65536))
            except OSError: break
    keys = b'?\x1b[15~\x10/\t\x1b[Z\x1b q\x03\x1b[A\x1b[Bjk\r\x7f\x1b[C\x1b[DleoRr.gmdfFnNw sTBC L12cQxDiv\x13\x02'
    os.write(master, keys)
    time.sleep(.3)
    os.write(master, b'q\r')
    try: proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        proc.kill(); proc.wait(timeout=2)
    os.close(master)
    if b'Yoctui' not in raw or proc.returncode not in (0, 1, -9):
        raise SystemExit(f'keyboard PTY failed: returncode={proc.returncode}')
print('real PTY keyboard matrix passed')
PY
