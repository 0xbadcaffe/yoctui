#!/usr/bin/env python3
"""Exercise portable validation paths without Docker, a daemon or a real build."""
import importlib.util
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import types
import unittest
from unittest.mock import MagicMock, patch

SCRIPTS = Path(__file__).resolve().parent


def load(name):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PortableValidationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name).resolve()
        self.home = self.base / 'operator home'
        self.default = self.home / 'src/yoctui-zcu102-2026.1'
        self.override = self.base / 'separate validation'
        for root in (self.default, self.override):
            (root / 'build/conf').mkdir(parents=True)
            (root / 'build/conf/bblayers.conf').touch()
            (root / 'setupsdk').touch()
        self.env = {'HOME': str(self.home)}

    def test_shell_helpers_preserve_default_and_explicit_roots_with_spaces(self):
        repo = self.base / 'repo'
        binary = repo / 'target/release/yoctui'
        binary.parent.mkdir(parents=True)
        binary.write_text('#!/bin/sh\nexit 0\n')
        binary.chmod(0o755)
        mockbin = self.base / 'bin'
        mockbin.mkdir()
        git = mockbin / 'git'
        git.write_text('#!/bin/sh\nprintf "%s\\n" "$MOCK_REPO"\n')
        git.chmod(0o755)
        docker = mockbin / 'docker'
        docker.write_text('#!' + sys.executable + '\nimport json,sys\n'
                          'if "readlink" in sys.argv:\n'
                          '    print(sys.argv[-1])\n'
                          'else:\n'
                          '    print("true" if sys.argv[1] == "inspect" else json.dumps(sys.argv[1:]))\n')
        docker.chmod(0o755)
        for name in ('live-zcu102.sh', 'prepare-zcu102-metadata.sh'):
            for root, override in ((self.default, {}),
                                   (self.override, {'YOCTUI_ZCU102_ROOT': str(self.override)})):
                with self.subTest(helper=name, root=root):
                    env = {**os.environ, **self.env,
                           'PATH': str(mockbin) + os.pathsep + os.environ['PATH'],
                           'MOCK_REPO': str(repo), 'YOCTUI_ZCU102_CONTAINER': 'test-container'}
                    env.pop('YOCTUI_ZCU102_ROOT', None)
                    env.update(override)
                    result = subprocess.run(['bash', str(SCRIPTS / name)], env=env,
                                            capture_output=True, text=True, check=True)
                    args = json.loads(result.stdout)
                    self.assertEqual(args[args.index('--workdir') + 1], str(root))
                    self.assertIn(str(root / 'setupsdk'), args)
                    self.assertIn(str(root / 'build'), args)
                    self.assertIn('test-container', args)

    def test_metadata_preflight_uses_container_canonical_binary_path(self):
        repo = self.base / 'repo'
        binary = repo / 'target/release/yoctui'
        binary.parent.mkdir(parents=True)
        binary.write_text('#!/bin/sh\nexit 0\n')
        binary.chmod(0o755)
        mockbin = self.base / 'bin'
        mockbin.mkdir()
        git = mockbin / 'git'
        git.write_text('#!/bin/sh\nprintf "%s\\n" "$MOCK_REPO"\n')
        git.chmod(0o755)
        docker = mockbin / 'docker'
        docker.write_text('#!' + sys.executable + '\nimport json,os,sys\n'
                          'if "readlink" in sys.argv:\n'
                          '    print(os.environ["CONTAINER_BINARY"])\n'
                          'else:\n'
                          '    print(json.dumps(sys.argv[1:]))\n')
        docker.chmod(0o755)
        canonical = str(self.base / 'container cache/release/yoctui')
        env = {**os.environ, **self.env,
               'PATH': str(mockbin) + os.pathsep + os.environ['PATH'],
               'MOCK_REPO': str(repo), 'YOCTUI_ZCU102_ROOT': str(self.override),
               'YOCTUI_ZCU102_CONTAINER': 'test-container',
               'CONTAINER_BINARY': canonical}
        result = subprocess.run(['bash', str(SCRIPTS / 'prepare-zcu102-metadata.sh')],
                                env=env, capture_output=True, text=True, check=True)
        args = json.loads(result.stdout)
        handoff = next(arg for arg in args if arg.startswith('OE_TERMINAL_CUSTOMCMD='))
        self.assertEqual(handoff, f"OE_TERMINAL_CUSTOMCMD='{canonical}' __menuconfig-handoff "
                         f"--socket '{self.override}/runtime/yoctui/menuconfig.sock' -- {{command}}")
        for invalid in ('relative/yoctui', canonical + "'", canonical + '\nother'):
            with self.subTest(binary=invalid):
                result = subprocess.run(
                    ['bash', str(SCRIPTS / 'prepare-zcu102-metadata.sh')],
                    env={**env, 'CONTAINER_BINARY': invalid},
                    capture_output=True, text=True)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, '')

    def test_preseed_default_and_override_preserve_exact_cache_and_pin(self):
        module = load('preseed-zcu102-kernel-cache')
        bb = types.ModuleType('bb')
        bb.utils = types.SimpleNamespace(lockfile=object(), unlockfile=object())
        for root, override in ((self.default, {}),
                               (self.override, {'YOCTUI_ZCU102_ROOT': str(self.override)})):
            with self.subTest(root=root), patch.dict(os.environ, {**self.env, **override}, clear=True), \
                    patch.object(sys, 'argv', ['preseed']), patch.object(sys, 'path', sys.path.copy()), \
                    patch.dict(sys.modules, {'bb': bb, 'bb.utils': bb.utils}), \
                    patch.object(module, 'install', return_value='SKIPPED') as install:
                module.main()
                self.assertEqual(sys.path[0], str(root / 'sources/poky/bitbake/lib'))
                install.assert_called_once_with(
                    root / 'inspection/linux-xlnx-pinned.git',
                    root / 'build/downloads/git2/github.com.Xilinx.linux-xlnx.git',
                    module.REVISION, module.BRANCH, module.REMOTE,
                    bb.utils.lockfile, bb.utils.unlockfile)

    def test_inspector_default_and_override_keep_socket_and_build_authority(self):
        module = load('inspect-zcu102-build')
        for root, override in ((self.default, {}),
                               (self.override, {'YOCTUI_ZCU102_ROOT': str(self.override)})):
            connection = MagicMock()
            connection.getsockopt.return_value = struct.pack('3i', 1, os.getuid(), os.getgid())
            with self.subTest(root=root), patch.dict(os.environ, {**self.env, **override}, clear=True), \
                    patch.object(module.socket, 'socket') as socket, \
                    patch.object(module, 'inspect', return_value={}) as inspect:
                socket.return_value.__enter__.return_value = connection
                module.main()
                connection.connect.assert_called_once_with(str(root / 'runtime/yoctui/daemon.sock'))
                inspect.assert_called_once_with(connection, root / 'build')

    def test_relative_and_symlink_roots_rejected_before_io(self):
        link = self.base / 'alias'
        link.symlink_to(self.override, target_is_directory=True)
        for name in ('preseed-zcu102-kernel-cache', 'inspect-zcu102-build'):
            module = load(name)
            for root in ('relative/path', str(link)):
                with self.subTest(helper=name, root=root), \
                        patch.dict(os.environ, {'YOCTUI_ZCU102_ROOT': root}), \
                        patch.object(sys, 'argv', [name]), self.assertRaisesRegex(ValueError, 'canonical'):
                    module.main()

    def test_preseed_rejects_missing_and_foreign_owned_roots(self):
        module = load('preseed-zcu102-kernel-cache')
        with patch.dict(os.environ, {'YOCTUI_ZCU102_ROOT': str(self.base / 'missing')}), \
                patch.object(sys, 'argv', ['preseed']), self.assertRaisesRegex(ValueError, 'owned'):
            module.main()
        with patch.dict(os.environ, {'YOCTUI_ZCU102_ROOT': str(self.override)}), \
                patch.object(sys, 'argv', ['preseed']), \
                patch.object(module.os, 'getuid', return_value=self.override.stat().st_uid + 1), \
                self.assertRaisesRegex(ValueError, 'owned'):
            module.main()

    def test_inspector_rejects_foreign_daemon_peer(self):
        module = load('inspect-zcu102-build')
        connection = MagicMock()
        connection.getsockopt.return_value = struct.pack('3i', 1, os.getuid() + 1, os.getgid())
        with patch.dict(os.environ, self.env, clear=True), \
                patch.object(module.socket, 'socket') as socket, \
                patch.object(module, 'inspect') as inspect, \
                self.assertRaisesRegex(ValueError, 'peer'):
            socket.return_value.__enter__.return_value = connection
            module.main()
        inspect.assert_not_called()


if __name__ == '__main__':
    unittest.main()
