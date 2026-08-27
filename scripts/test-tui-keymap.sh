#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
cargo build -p yoctui >/dev/null
python3 - "$repo_root" <<'PY'
import os, pty, select, struct, subprocess, sys, termios, fcntl, time
root = sys.argv[1]
with __import__('tempfile').TemporaryDirectory(prefix='yoctui-keymap-', dir='/tmp') as tmp:
    isolated_env = os.environ.copy()
    for variable, directory in (
        ('XDG_CONFIG_HOME', 'config'),
        ('XDG_STATE_HOME', 'state'),
        ('XDG_RUNTIME_DIR', 'runtime'),
    ):
        path = os.path.join(tmp, directory)
        os.mkdir(path, mode=0o700)
        isolated_env[variable] = path
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
    def become_session_leader():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)
    proc = subprocess.Popen([os.path.join(root, 'target/debug/yoctui'), '--backend', 'process', '--no-color'], stdin=slave, stdout=slave, stderr=slave, preexec_fn=become_session_leader, env=isolated_env)
    os.close(slave)
    raw = bytearray()
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline and b'yoctui' not in raw.lower():
        ready, _, _ = select.select([master], [], [], .2)
        if ready:
            try: raw.extend(os.read(master, 65536))
            except OSError: break
    os.write(master, b'\x1b')
    time.sleep(.2)
    keys = b'?\x1b[15~\x10/\t\x1b[Z\x1b q\x03\x1b[A\x1b[Bjk\r\x7f\x1b[C\x1b[DleoRr.gmdfFnNw sTBC L12cQxDiv\x13\x02'
    os.write(master, keys)
    matrix_deadline = time.monotonic() + 1
    while time.monotonic() < matrix_deadline:
        ready, _, _ = select.select([master], [], [], .1)
        if ready:
            try: raw.extend(os.read(master, 65536))
            except OSError: break
    # Resolve a possible terminal-prefix wait, then unwind nested modal layers
    # with distinct Escape events so they cannot be decoded as Alt+q.
    os.write(master, b'\x02')
    for _ in range(4):
        os.write(master, b'\x1b')
        time.sleep(.15)
    os.write(master, b'q')
    try: proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        proc.kill(); proc.wait(timeout=2)
    while True:
        ready, _, _ = select.select([master], [], [], .1)
        if not ready: break
        try: raw.extend(os.read(master, 65536))
        except OSError: break
    os.close(master)
    if (b'yoctui' not in raw.lower() or b'\x1b[?1049h' not in raw
            or b'\x1b[?1049l' not in raw or b'\xef\xbf\xbd' in raw
            or proc.returncode != 0):
        raise SystemExit(f'keyboard PTY failed: returncode={proc.returncode}')
print('real PTY keyboard matrix passed')
PY
