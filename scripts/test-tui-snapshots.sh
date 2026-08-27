#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
cargo build -p yoctui >/dev/null
python3 - "$repo_root" <<'PY'
import atexit, os, pty, select, struct, subprocess, sys, termios, fcntl, time, tempfile, re
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
        binary = os.path.join(root, 'target/debug/yoctui')
        build_dir = os.path.join(tmp, 'build')
        daemon_prefix = [binary, '--backend', 'process', '--build-dir', build_dir, 'daemon']
        subprocess.run(
            daemon_prefix + ['start'],
            check=True,
            env=isolated_env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        stop_daemon = daemon_prefix + ['stop']
        atexit.register(
            lambda command=stop_daemon, environment=isolated_env: subprocess.run(
                command,
                env=environment,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                check=False,
            )
        )
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        proc = subprocess.Popen([binary, '--backend', 'process', '--build-dir', build_dir, '--no-color'], stdin=slave, stdout=slave, stderr=slave, start_new_session=True, env=isolated_env)
        os.close(slave)
        raw = bytearray(); deadline = time.monotonic() + 8
        while time.monotonic() < deadline and b'yoctui' not in raw.lower():
            ready, _, _ = select.select([master], [], [], .2)
            if ready:
                try: raw.extend(os.read(master, 65536))
                except OSError: break
        # A private first-run session opens onboarding by design. Dismiss it
        # before routing snapshot navigation; Esc is inert if onboarding was
        # already completed by a future fixture.
        os.write(master, b'\x1b')
        dismiss_deadline = time.monotonic() + .5
        while time.monotonic() < dismiss_deadline:
            ready, _, _ = select.select([master], [], [], .1)
            if ready:
                try: raw.extend(os.read(master, 65536))
                except OSError: break
        if name == 'wide':
            # xterm-compatible F2: enter the canonical Tasks workbench before
            # capturing the literal-reference integration anchors.
            os.write(master, b'\x1bOQ')
            task_deadline = time.monotonic() + 2
            while time.monotonic() < task_deadline:
                ready, _, _ = select.select([master], [], [], .2)
                if ready:
                    try: raw.extend(os.read(master, 65536))
                    except OSError: break
        os.write(master, b'q\r')
        try: proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill(); proc.wait(timeout=2)
        os.close(master)
        subprocess.run(
            stop_daemon,
            env=isolated_env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        text = bytes(raw).decode('utf-8', 'replace')
        normalized = re.sub(r'\x1b\[[0-9;?]*[ -/]*[@-~]', '', text)
        normalized = '\n'.join(line.rstrip() for line in normalized.splitlines() if line.strip())
        open(os.path.join(artifact, f'{name}.txt'), 'w').write(normalized[-32768:])
        if 'yoctui' not in normalized.lower() or proc.returncode not in (0, 1, -9):
            raise SystemExit(f'snapshot failed at {name}: returncode={proc.returncode}')
        if name == 'wide':
            for anchor in ('Tasks: Build', 'F1 Help', 'F10 Menu'):
                if anchor not in normalized:
                    raise SystemExit(f'wide reference snapshot missing {anchor!r}')
print('PTY semantic snapshots passed')
PY
