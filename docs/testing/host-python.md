# Persistent host Python repair

On 2026-09-07, OpenBMC Romulus stopped advancing at 563/6,812 tasks. Several
Python and Git-wrapper processes repeatedly executed pyenv instead of their
payload. `tmp/hosttools/python3` pointed to the pyenv shim; BitBake removed
ordinary system directories from PATH, so pyenv's system fallback found that
same shim again. This matches
[upstream issue 2696](https://github.com/pyenv/pyenv/issues/2696).

The user explicitly requested a persistent global repair. Scope is the
`bspguy-dev` account's default shells and its discovered Yocto host-tool links;
no distribution-owned interpreter, other user's configuration, virtualenv,
source tree or package output was deleted or replaced.

## Applied changes

- `.bashrc` no longer automatically enables pyenv shims. Pyenv remains on PATH
  for explicit use. `.profile` and `.bashrc` source
  `.config/shell/python-path.sh`, which removes inherited pyenv shim entries
  while preserving unrelated PATH entries and active virtualenv precedence.
  The Bash guard runs before its noninteractive early return too.
- Pyenv upgraded from v2.6.16 (`e805257`) to upstream v2.8.5
  (`c293de3650a2d08ea36c5f6f0d55c5a3841b0c72`), followed by `pyenv rehash`.
  The release contains the
  [upstream shim-alias recursion fix](https://github.com/pyenv/pyenv/pull/3375).
  Its checkout was clean before and after the upgrade. Python 3.10.14 and
  3.11.9 remain installed and work with explicit `PYENV_VERSION` selection.
- Inactive OpenBMC `tmp/hosttools/python3` now links to `/usr/bin/python3`.
  Existing Poky already resolved to system Python and was not modified.
  Discovery under `~/src` and `~/projects` found these two hosttools directories.
- OpenBMC image job 1 was cancelled through Yoctui before repair. The command
  acknowledgement took 282 ms in this one-shot debug-binary observation;
  terminal outcome was unsuccessful, not image success. The private daemon
  was then stopped; no Cooker or Worker remained.

## Verification

Login and interactive Bash started with an inherited shim-first PATH both
resolve `python3` to `/usr/bin/python3` (Python 3.14.4). A restricted PATH
containing only OpenBMC hosttools runs Python successfully within five seconds.
A separate shim alias prepended to that restricted PATH now recovers to the
real hosttools interpreter rather than looping. Explicit pyenv 3.10.14 and
3.11.9 invocations both report their correct version. A synthetic virtualenv
PATH remains first after duplicate inherited shim entries are removed.
Shell syntax validation passes. These are host repair checks, not image-build
completion or Yoctui release-performance evidence.

For an existing terminal:

```sh
. ~/.bashrc
hash -r
command -v python3
```

For an intentionally selected managed interpreter:

```sh
PYENV_VERSION=3.11.9 pyenv exec python3 --version
```

Already-running processes cannot have their inherited environment changed;
restart build daemons from a fresh shell. Manually re-enabling pyenv shims before
initializing a new Yocto workspace can still make it capture a shim, so use
the unshimmed default for builds. The upstream fix prevents recursion; it is
not a replacement for selecting a real host interpreter.

## Recovery

Original `.bashrc`, `.profile` and OpenBMC Python link are preserved at
`/home/bspguy-dev/.local/state/yoctui-host-python-backup.p4JYl8`.
Pyenv's previous revision is retained in branch
`yoctui-host-python-before-20260907`. Restoring the old configuration/link can
restore the original build failure; do not do so while a build is running.
The small alias test fixture is `/tmp/yoctui-python-alias-test.tyOOoG`.
