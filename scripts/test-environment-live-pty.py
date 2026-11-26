#!/usr/bin/env python3
"""Real path/discovery/BitBake check; explicit paths authorize idle daemon reload.

Does not clean files, alter configuration, or start an image build.
"""
import argparse
import json
import os
from pathlib import Path
import runpy
import subprocess

from pty_acceptance import start_terminal

ROOT = Path(__file__).resolve().parents[1]
HELPERS = runpy.run_path(str(ROOT / 'scripts/test-environment-reconnect-pty.py'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--build', type=Path, required=True)
    parser.add_argument('--capture-dir', type=Path, required=True)
    args = parser.parse_args()
    source, build = args.source.resolve(), args.build.resolve()
    script = source / 'oe-init-build-env'
    assert script.is_file() and (build / 'conf/local.conf').is_file()
    args.capture_dir.mkdir(parents=True, exist_ok=True)
    env = {key: value for key, value in os.environ.items()
           if not key.startswith('YOCTUI_') and key not in
           {'BUILDDIR', 'BBPATH', 'BBSERVER', 'TEMPLATECONF', 'OEROOT', 'PYTHONPATH'}}
    env['TERM'] = 'xterm-256color'
    env['YOCTUI_TERMINAL_GRAPHICS'] = 'none'
    terminal = start_terminal(ROOT, env, 180, 52, '--backend', 'bridge', '--no-color')

    def send(keys):
        os.write(terminal.master, keys)

    def capture(name):
        (args.capture_dir / (name + '.txt')).write_text(terminal.screen.text())
        (args.capture_dir / (name + '.cells')).write_text(terminal.screen.cell_golden())

    def edit(value, typed=False):
        send(b'e')
        terminal.wait_for('Enter accept value')
        data = str(value).encode()
        if typed:
            send(data[:5])
            data = data[5:]
        send(b'\x1b[200~' + data + b'\x1b[201~')
        send(b'\r')
        terminal.wait_for('s save profile', absent='Enter accept value')
        terminal.wait_for(str(value))

    try:
        try:
            terminal.wait_for("Six-step workbench path", timeout=1)
        except AssertionError:
            terminal.wait_for("Quit")
        else:
            send(b"\x1b")
            terminal.wait_for("Quit", absent="Six-step workbench path")
        HELPERS['environment_screen'](terminal)
        send(b'e')
        terminal.wait_for('Configure build environment')
        edit('relative/source')
        send(b's')
        terminal.wait_for('must be absolute normal paths')
        capture('relative-path-error')
        edit(source, typed=True)
        send(b'b')
        terminal.wait_for('Browse directories', absent='Loading directories')
        terminal.wait_for(str(source))
        send(b's')
        terminal.wait_for('Configure build environment')
        terminal.wait_for(str(script))
        capture('auto-discovered-script')
        send(b'\t')
        edit(build)
        send(b'\t')
        edit(source / 'missing-init-script')
        send(b's')
        terminal.wait_for('missing build environment path:')
        terminal.wait_for('missing-init-script')
        capture('missing-script-error')
        send(b'\x1b')
        terminal.wait_for('Enter/e Configure paths')
        HELPERS['set_paths'](terminal, [source, build, script])
        send(b'\r')
        terminal.wait_for('Loaded environment', timeout=720)
        terminal.wait_for(str(build))
        capture('connected')
        send(b'\x1b[18~')
        terminal.wait_for('Recipe: ', timeout=720)
        assert 'Recipes (shown: 0 of 0)' not in terminal.screen.text()
        capture('real-recipes')
        send(b'\x1b[19~')  # F8 Images
        terminal.wait_for('Images')
        send(b'\t')
        send(b'i')
        terminal.wait_for('Available image targets', timeout=60)
        terminal.wait_for('core-image-minimal')
        capture('image-targets')
        send(b'\x1b')
        terminal.wait_for('Tab switches', absent='Available image targets')
        terminal.finish()
        result = {'status': 'REAL PASS', 'scope': 'native UI, typing/bracketed paste, invalid path diagnostics, initializer auto-discovery, real BitBake metadata and image targets; no image build',
                  'source_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                  'source': str(source), 'build': str(build), 'initializer': str(script)}
        (args.capture_dir / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))
    except BaseException:
        capture('failure')
        raise
    finally:
        terminal.cleanup()
        os.close(terminal.master)


if __name__ == '__main__':
    main()
