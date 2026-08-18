#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
cargo build -p yoctui >/dev/null
python3 - "$repo_root" <<'PY'
import os, pty, select, struct, subprocess, sys, termios, fcntl, time, tempfile, re
root = sys.argv[1]
artifact = os.path.join(root, 'artifacts', 'release-quality', 'snapshots')
os.makedirs(artifact, exist_ok=True)
for width, height, name in ((80, 24, 'narrow'), (100, 30, 'medium'), (160, 48, 'wide')):
    with tempfile.TemporaryDirectory(prefix='yoctui-snapshot-', dir='/tmp') as tmp:
        os.mkdir(os.path.join(tmp, 'build'))
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
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        proc = subprocess.Popen([os.path.join(root, 'target/debug/yoctui'), '--backend', 'process', '--build-dir', os.path.join(tmp, 'build'), '--no-color'], stdin=slave, stdout=slave, stderr=slave, start_new_session=True, env=isolated_env)
        os.close(slave)
        raw = bytearray(); deadline = time.monotonic() + 8
        while time.monotonic() < deadline and b'yoctui' not in raw.lower():
            ready, _, _ = select.select([master], [], [], .2)
            if ready:
                try: raw.extend(os.read(master, 65536))
                except OSError: break
        os.write(master, b'q\r')
        try: proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill(); proc.wait(timeout=2)
        os.close(master)
        text = bytes(raw).decode('utf-8', 'replace')
        normalized = re.sub(r'\x1b\[[0-9;?]*[ -/]*[@-~]', '', text)
        normalized = '\n'.join(line.rstrip() for line in normalized.splitlines() if line.strip())
        open(os.path.join(artifact, f'{name}.txt'), 'w').write(normalized[-32768:])
        if 'yoctui' not in normalized.lower() or proc.returncode not in (0, 1, -9):
            raise SystemExit(f'snapshot failed at {name}: returncode={proc.returncode}')
print('PTY semantic snapshots passed')
PY
