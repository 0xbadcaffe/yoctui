#!/usr/bin/env python3
"""Install a verified pinned shallow cache without overriding a BitBake fetch.

Prepare inspection/linux-xlnx-pinned.git with a depth-one fetch of the exact
recipe SRCREV first. This helper never fetches, changes recipes or task stamps,
replaces a cache, or builds an image. BitBake still runs fetch/unpack normally.
"""
from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys

ROOT = Path('/home/bspguy-dev/src/yoctui-zcu102-2026.1')
REVISION = '4f7afe14f7246986ca858d9a0880f5db6ba02a4b'
BRANCH = 'xlnx_rebase_v6.18_LTS'
REMOTE = 'https://github.com/Xilinx/linux-xlnx.git'


def git(stage: Path, *arguments: str) -> str:
    return subprocess.check_output(
        ['git', '-c', 'safe.bareRepository=all', '-C', str(stage), *arguments],
        text=True, stderr=subprocess.PIPE, timeout=120,
    ).strip()


def install(stage, cache, revision, branch, remote, lock, unlock):
    stage, cache = Path(stage), Path(cache)
    for path in (stage, cache.parent):
        if path.resolve() != path or not path.is_dir():
            raise ValueError('Cache and stage must use real normalized directories')
        if path.stat().st_uid != os.getuid():
            raise ValueError('Cache and stage must belong to the current user')
    if git(stage, 'rev-parse', '--is-bare-repository') != 'true':
        raise ValueError('Stage must be a bare repository')
    if git(stage, 'rev-parse', '--is-shallow-repository') != 'true':
        raise ValueError('Stage must be shallow')
    if git(stage, 'rev-parse', f'refs/heads/{branch}') != revision:
        raise ValueError('Stage revision differs from the exact recipe pin')
    if git(stage, 'remote', 'get-url', 'origin') != remote:
        raise ValueError('Stage remote differs from the recipe source')
    git(stage, 'fsck', '--full', '--no-reflogs')
    # Use the vendored BitBake lock implementation, including inode/race checks.
    # git.py localpath is the clone directory; FetchData adds .lock to it.
    held = lock(str(cache) + '.lock', retry=False)
    if held is None:
        return 'SKIPPED: BitBake fetch lock is busy; cache and stage untouched'
    try:
        if os.path.lexists(cache):
            return 'SKIPPED: cache already exists; cache and stage untouched'
        os.rename(stage, cache)
        return f'INSTALLED: exact shallow {revision}; task stamps unchanged'
    finally:
        unlock(held)


def main():
    if len(sys.argv) != 1:
        raise SystemExit('This helper has no overrides; it is scoped to ZCU102 validation')
    sys.path.insert(0, str(ROOT / 'sources/poky/bitbake/lib'))
    import bb.utils
    print(install(
        ROOT / 'inspection/linux-xlnx-pinned.git',
        ROOT / 'build/downloads/git2/github.com.Xilinx.linux-xlnx.git',
        REVISION, BRANCH, REMOTE, bb.utils.lockfile, bb.utils.unlockfile,
    ))


if __name__ == '__main__':
    main()
