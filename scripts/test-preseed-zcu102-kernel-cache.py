#!/usr/bin/env python3
"""Focused local Git checks; no network, image build or daemon mutation."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('preseed', Path(__file__).with_name('preseed-zcu102-kernel-cache.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class CacheTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        original = self.root / 'source'
        subprocess.run(['git', 'init', '-q', '--initial-branch=test', str(original)], check=True)
        subprocess.run(['git', '-C', str(original), '-c', 'user.name=Roy Cohen', '-c', 'user.email=roy@0xbadcaffe.dev', 'commit', '-q', '--allow-empty', '-m', 'fixture'], check=True)
        self.stage = self.root / 'stage.git'
        self.remote = original.as_uri()
        subprocess.run(['git', 'clone', '-q', '--bare', '--depth=1', '--branch=test', self.remote, str(self.stage)], check=True)
        self.revision = module.git(self.stage, 'rev-parse', 'refs/heads/test')
        self.cache = self.root / 'cache.git'
        self.unlocked = []

    def install(self, **overrides):
        options = dict(stage=self.stage, cache=self.cache, revision=self.revision, branch='test', remote=self.remote, lock=lambda *_a, **_k: 'held', unlock=self.unlocked.append)
        options.update(overrides)
        return module.install(**options)

    def test_install_exact_shallow_source_and_release_lock(self):
        self.assertTrue(self.install().startswith('INSTALLED'))
        self.assertFalse(self.stage.exists())
        self.assertEqual(module.git(self.cache, 'rev-parse', 'test'), self.revision)
        self.assertEqual(self.unlocked, ['held'])
        self.assertFalse(Path(str(self.cache) + '.done').exists())

    def test_existing_cache_not_replaced(self):
        self.cache.mkdir()
        identity = self.cache.stat().st_ino
        self.assertIn('already exists', self.install())
        self.assertEqual(self.cache.stat().st_ino, identity)
        self.assertTrue(self.stage.exists())
        self.assertEqual(self.unlocked, ['held'])

    def test_busy_fetch_lock_leaves_stage_and_cache_untouched(self):
        self.assertIn('lock is busy', self.install(lock=lambda *_a, **_k: None))
        self.assertTrue(self.stage.exists())
        self.assertFalse(self.cache.exists())
        self.assertEqual(self.unlocked, [])

    def test_wrong_revision_or_remote_rejected_before_lock(self):
        for options in (dict(revision='0' * 40), dict(remote='file:///wrong-source')):
            with self.assertRaises(ValueError):
                self.install(**options)
        self.assertTrue(self.stage.exists())
        self.assertFalse(self.cache.exists())
        self.assertEqual(self.unlocked, [])

    def test_symlink_stage_rejected(self):
        link = self.root / 'link.git'
        os.symlink(self.stage, link)
        with self.assertRaises(ValueError):
            self.install(stage=link)
        self.assertTrue(self.stage.exists())
        self.assertFalse(self.cache.exists())


if __name__ == '__main__':
    unittest.main()
