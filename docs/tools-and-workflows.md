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
syntax-highlighted editor and daemon-owned terminals.
The explorer loads one directory at a time, so opening a kernel checkout does not
require scanning the complete tree. Devtool source-workspace actions reopen the
matching Workbench session without discarding unfinished buffers.

Use **Alt+m** for folder/file creation, recipe binding, build, modify, status,
devshell, workspace shell, changes, patch creation, finish and reset. Recipe
commands require the session's original build environment to be attached. Build
output stays in the bottom panel; **Alt+t** selects the shell and **Alt+z** expands
it for interactive tools. **Alt+c** reviews `bitbake <recipe> -c compile`, loading
the reported task inventory first when needed. **Alt+b** reviews a regular recipe
build; **Alt+u** opens a prefilled `devtool build` form. **Alt+d** reviews Devtool
modify. Save edited source tabs before running these commands.
After Modify succeeds, its reported source workspace opens automatically when
the same recipe/build is still selected; switching elsewhere does not interrupt
your current work.

**Alt+w** shows spaces as `·` and tabs as `→` with four-column tab stops. This is
display only; saves preserve the original spaces and tabs. Mouse wheel and the
scrollbar move the view independently of the cursor. Page Up/Down uses the actual
editor height. **Ctrl+f** finds text, **Ctrl+g** (or Alt+n) advances through
highlighted matches and **Alt+p** goes back;
clear the Find field and confirm to clear the search. **Alt+v** lists open tabs
with paths and dirty markers; Enter activates one, Delete closes a saved tab.
Alt+Left/Right cycles tabs, Alt+1..9 selects a tab, and Ctrl+W closes a saved tab.

For recipe debugging, **Alt+i** opens `devtool ide-sdk` with the bound recipe,
`--ide none` and `--no-strip`. Review and run it to prepare the recipe toolchain.
**Alt+g** (or F5 in the editor) opens GDB settings: local/remote mode, GDB or
cross-GDB executable, matching unstripped ELF, optional target sysroot, remote
gdbserver host:port, and optional compiled-source prefix to map to the workspace.
Select an ELF in the explorer before Alt+g to prefill its path. Settings are
retained with the workspace. The existing terminal review shows the complete argv
before launching a daemon-owned GDB terminal.

Click the line-number gutter or press **Alt+k/F9** to stage a file/line breakpoint.
Markers follow line insertions/deletions and persist with saved workspaces.
They are applied when GDB launches; changing a marker does not modify a running
debugger. In local mode use `run`; in remote mode start the matching program under
`gdbserver` on the target first, then connect and use `continue`. GDB provides
`next`, `step`, `bt`, `info registers` and `list`. **Alt+e** returns to the editor;
**Alt+t** reopens the running debugger. Remote attach can pause the target.
Kernel-managed QEMU debugging remains available from the Kernel screen.

Folders, editor buffers and cursor positions are recovered from
`${XDG_STATE_HOME:-$HOME/.local/state}/yoctui/workbenches.json`. The private,
atomically replaced file includes unsaved text: do not share it without reviewing
its contents. Another client cannot overwrite a recovery store held by an open
Yoctui client. Source saves detect external changes and retain the buffer on
conflict; they do not silently overwrite another editor's work. Ctrl+S writes a
source file; automatic recovery saves do not write the source or publish patches.

Limits: 16 sessions, eight folders/session, 16 open files/session, 1 MiB per text
file, 128 staged breakpoints/session and 8 MiB total reserved recovery text. The
editor rejects binary files and symlink traversal. Session/folder removal only forgets metadata; dirty tabs must be saved
before removal. Terminal process ownership remains with the daemon; closing the
client does not terminate workspace shells. See [keyboard controls](keyboard-shortcuts.md#development-workbench).

Adapters need capability/action entries, bounded shell-free runner tests, cancellation,
and this table. verify-utility-coverage.sh --catalog-only checks coverage;
fixtures do not establish every [live workflow](compatibility.md).
