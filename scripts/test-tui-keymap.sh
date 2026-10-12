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
    # Assert each current route before sending another input. Arbitrary batched
    # keys can start dialogs/chords and consume the eventual quit request; they
    # never proved the individual shortcuts worked. Rust keymap tests cover the
    # complete catalog; this is a state-checked real terminal routing smoke test.
    def press(key, expected, absent=None):
        os.write(master, key)
        terminal.wait_for(expected, absent=absent)
    try:
        press(b'\x1bOP', 'Keyboard and action reference')
        press(b'g', 'Six-step workbench path')
        press(b'\x1b', 'Keyboard and action reference', 'Six-step workbench path')
        press(b'\x1b', 'Build Overview', 'Keyboard and action reference')
        press(b'\x1b[15~', 'Log Viewer')
        press(b'\x10', 'Command Palette')
        press(b'Open Diff Tool', 'Open Diff Tool')
        press(b'\r', 'Diff tool · read-only', 'Command Palette')
        press(b'\x1b[15~', 'Mark one file on each side with Space')
        press(b'\x1b', 'h/l groups')
        press(b'\x1b[15~', 'Log Viewer')
        press(b'\x1b[24~', 'Yoctui Application Menu')
        press(b'\x1b', 'Log Viewer', 'Yoctui Application Menu')
        press(b'\x02?', 'Keyboard and action reference')
        press(b'\x1b', 'Build Overview', 'Keyboard and action reference')
        terminal.finish()
    finally:
        terminal.cleanup()
    os.close(master)
    if (b'yoctui' not in raw.lower() or b'\x1b[?1049h' not in raw
            or b'\x1b[?1049l' not in raw or b'\xef\xbf\xbd' in raw
            or proc.returncode != 0):
        raise SystemExit(f'keyboard PTY failed: returncode={proc.returncode}')
print('state-checked real PTY keyboard routing passed')
PY
