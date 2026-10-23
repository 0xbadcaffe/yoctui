# Tools and workflows

Availability comes from the initialized build's daemon capability snapshot.
Unknown/missing tools remain visible with reasons. Typed forms and expert argv
use shell-free review; mutations require confirmation. Host PATH alone is insufficient.

| Tool family | Workflow |
| --- | --- |
| `oe-init-build-env`, `oe-setup-builddir`, `oe-buildenv-internal` | Environment setup/internal information. |
| `bitbake` | Reviewed targets/tasks/options, events, cancellation. |
| `devtool`, `recipetool` | Recipe/source/layer forms; capability-gated expert arguments. |
| `bitbake-layers` | Layer queries and confirmed mutations. |
| `oe-pkgdata-util` | Generated package metadata. |
| `bitbake-getvar`, `bitbake-dumpsig`, `bitbake-diffsigs`, `dumpsig`, `diffsigs`, `whatchanged` | Variables/signature comparisons. |
| `runqemu`, `wic`, `runqemu-extract-sdk` | Artifact-bound launch/creation/device review. |
| `oe-find-native-sysroot`, `oe-run-native` | Reviewed native environment/argv. |
| `kas` | Installed expert checkout/build/shell/lock, network/mutation review. |
| `oe-selftest`, `bitbake-selftest`, `testimage`, `testsdk`, `ptest`, `resulttool` | Tests, results, comparison, JUnit. |
| CVE/SPDX/SBOM helpers | Exact report paths, explicit partial results. |
| `yocto-check-layer`, `yocto-layer`, `yocto-bsp`, `yocto-kernel` | Version-aware checks/expert operations. |
| `sstate-cache-management.sh` | Exact cleanup candidates and destructive review. |
| `buildhistory-diff`, `build-compare`, locked signatures | Exact source/cache evidence. |
| `oe-git-archive`, `create-pull-request`, `send-pull-request` | Local review, separate network confirmation. |
| PR/hash services, `toaster`, `pybootchartgui` | Information/managed diagnostics. |
| `bitbake-worker`, `bitbake-prserv`, `bitbake-hashserv` | Internal; not user-launchable. |

## Development Workbench

**Build → Workbench** provides named, multi-folder development sessions for
Devtool sources, Poky/layer code, recipes and scripts. It uses Yoctui's native
syntax-highlighted editor and daemon-owned terminals, not a bundled Go application.
The explorer loads one directory at a time, so opening a kernel checkout does not
require scanning the complete tree. Devtool source-workspace actions reopen the
matching Workbench session without discarding unfinished buffers.

Use **Alt+m** for folder/file creation, recipe binding, build, modify, status,
devshell, workspace shell, changes, patch creation, finish and reset. Recipe
commands require the session's original build environment to be attached. Build
output stays in the bottom panel; **Alt+t** selects the shell and **Alt+z** expands
it for interactive tools. Existing kernel debugging tools can also be used in
daemon terminals; this editor does not yet provide LSP or graphical breakpoint controls.

Folders, editor buffers and cursor positions are recovered from
`${XDG_STATE_HOME:-$HOME/.local/state}/yoctui/workbenches.json`. The private,
atomically replaced file includes unsaved text: do not share it without reviewing
its contents. Another client cannot overwrite a recovery store held by an open
Yoctui client. Source saves detect external changes and retain the buffer on
conflict; they do not silently overwrite another editor's work. Ctrl+S writes a
source file; automatic recovery saves do not write the source or publish patches.

Limits: 16 sessions, eight folders/session, 16 open files/session, 1 MiB per text
file and 8 MiB total reserved recovery text. Binary files and symlink traversal are
rejected. Session/folder removal only forgets metadata; dirty tabs must be saved
before removal. Terminal process ownership remains with the daemon; closing the
client does not terminate workspace shells. See [keyboard controls](keyboard-shortcuts.md#development-workbench).

Adapters need capability/action entries, bounded shell-free runner tests, cancellation,
and this table. verify-utility-coverage.sh --catalog-only checks coverage;
fixtures do not establish every [live workflow](compatibility.md).
