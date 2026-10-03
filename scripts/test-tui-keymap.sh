#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
cargo build -p yoctui >/dev/null
python3 - "$repo_root" <<'PY'
import os, pty, select, struct, subprocess, sys, termios, fcntl, time
root = sys.argv[1]
sys.path.insert(0, os.path.join(root, 'scripts'))
from pty_acceptance import binary_path, isolated_environment, TerminalAcceptance
with __import__('tempfile').TemporaryDirectory(prefix='yoctui-keymap-', dir='/tmp') as tmp:
    isolated_env = isolated_environment(tmp)
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
    def become_session_leader():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)
    proc = subprocess.Popen([binary_path(root), '--backend', 'process', '--no-color'], stdin=slave, stdout=slave, stderr=slave, preexec_fn=become_session_leader, env=isolated_env)
    os.close(slave)
    raw = bytearray()
    terminal = TerminalAcceptance(master, proc, raw, 100, 30)
    terminal.dismiss_onboarding()
    keys = b'?\x1b[15~\x10/\t\x1b[Z\x1b q\x03\x1b[A\x1b[Bjk\r\x7f\x1b[C\x1b[DleoRr.gmdfFnNw sTBC L12cQxDiv\x13\x02'
    os.write(master, keys)
    matrix_deadline = time.monotonic() + 1
    while time.monotonic() < matrix_deadline:
        terminal.read()
    # Resolve a possible terminal-prefix wait, then unwind nested modal layers
    # with distinct Escape events so they cannot be decoded as Alt+q.
    os.write(master, b'\x02')
    terminal.unwind()
    terminal.finish()
    os.close(master)
    if (b'yoctui' not in raw.lower() or b'\x1b[?1049h' not in raw
            or b'\x1b[?1049l' not in raw or b'\xef\xbf\xbd' in raw
            or proc.returncode != 0):
        raise SystemExit(f'keyboard PTY failed: returncode={proc.returncode}')
print('real PTY keyboard matrix passed')
PY
