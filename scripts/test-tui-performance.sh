#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
cargo build -p yoctui >/dev/null
python3 - "$repo_root" <<'PY'
import json, os, pty, select, struct, subprocess, sys, termios, fcntl, time, tempfile
root = sys.argv[1]
reports = os.path.join(root, 'artifacts', 'release-quality', 'performance')
os.makedirs(reports, exist_ok=True)
rows = []
for width, height in ((80, 24), (160, 48)):
    with tempfile.TemporaryDirectory(prefix='yoctui-perf-', dir='/tmp') as tmp:
        os.mkdir(os.path.join(tmp, 'build'))
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        started = time.perf_counter()
        proc = subprocess.Popen([os.path.join(root, 'target/debug/yoctui'), '--backend', 'process', '--build-dir', os.path.join(tmp, 'build'), '--no-color'], stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        raw = bytearray(); first_frame = None; deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            ready, _, _ = select.select([master], [], [], .05)
            if ready:
                try: raw.extend(os.read(master, 65536))
                except OSError: break
                if first_frame is None and b'Yoctui' in raw: first_frame = time.perf_counter() - started
                if first_frame is not None: break
        os.write(master, b'q\r')
        try: proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill(); proc.wait(timeout=2)
        os.close(master)
        rows.append({'width': width, 'height': height, 'first_frame_seconds': first_frame, 'bytes': len(raw), 'returncode': proc.returncode})
if any(row['first_frame_seconds'] is None or row['first_frame_seconds'] > 8 for row in rows):
    raise SystemExit(f'performance budget exceeded: {rows}')
with open(os.path.join(reports, 'tui.json'), 'w') as handle:
    json.dump({'budgets': {'first_frame_seconds': 8, 'output_bytes': 262144}, 'samples': rows}, handle, indent=2)
print('TUI performance budgets passed')
PY
