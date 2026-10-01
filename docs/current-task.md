# Current Task

**ID:** KGDB-SERIAL-UI-001
**Title:** Review and launch KGDB serial clients in the Kernel workbench
**Status:** IN_PROGRESS

KGDB-SERIAL-PLAN-001 is DONE in v0.1.263: three model/four CLI tests, eight
QEMU backend regressions, formatting, strict Clippy, version policy and roadmap
pass. Add the appended technique, six-field form, typed config/GDB template
preview and correlated worker integration. Test trapped/narrow forms, cancellation,
stale/covered errors and unchanged QEMU/TCP-GDB/terminal lifecycle. Run the
registry's focused checks, bump version, build optimized release, commit/push and
rebuild bound to the final commit. Do not run the full suite or alter the board.
KGDB-SERIAL-LIVE-001 separately needs a compatible board and exact inputs.

Previous user fixes (completed):

MODIFIER-SHORTCUTS-001 is DONE in v0.1.261 with focused app/model/UI/CLI
verification, strict Clippy, formatting, version policy and roadmap passing.
EDITOR-GITUI-CONTEXT-001 is DONE in v0.1.262 with exact-context bounded
asynchronous read-only inspection, correlated cancellation, retained dirty
editor state, contextual F12 routing and Ctrl+B then e restoration. Focused
model/app/UI/CLI checks, real GitUI 0.28.1 input/resize/exit smoke, workspace,
RootFS, search, QEMU and saved-environment regressions pass. Formatting, strict
Clippy, UI contract, version policy, roadmap and optimized build pass. Final
push and commit-bound optimized rebuild deliver both tasks. No full suite ran.

The remaining external blocker is:

The retained performance evidence is not bound to the current source tree: it
has 143 previously documented source digest mismatches, including changes
predating M67. Supply a new genuine source/binary-bound Yocto 6.0.2 linux-yocto
compile capture using the documented release workload. Do not rewrite historical
digests or substitute fake-process startup timings for live evidence.

```bash
./scripts/verify-performance.sh --real-poky-evidence
./scripts/verify-completion.sh
```

This external prerequisite remains unresolved. The user
explicitly deferred the full suite; do not run the full completion suite without
a subsequent instruction. All M105/M106 requested corrections are DONE:
v0.1.258 target RootFS owner/group/mode, v0.1.259 systemd viewport scrolling,
and v0.1.260 single search activity marker. Focused verification and genuine
read-only Romulus ownership smoke pass. Seven optional wider UI assertions
fail identically on the previous commit and are documented, not weakened.
No overall completion/performance certification is claimed. Final push and
source-bound optimized binary handoff deliver these fixes; installed PATH
binary is still v0.1.250, so use target/release/yoctui explicitly.
