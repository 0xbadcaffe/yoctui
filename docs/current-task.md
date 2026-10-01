# Current Task

**ID:** KGDB-SERIAL-PLAN-001
**Title:** Validate a closed read-only KGDB serial attachment plan
**Status:** IN_PROGRESS

New user priority: continue the Kernel debugging roadmap with its next physical
board attach slice. Implement the closed typed spec, pure bounded kernel config
checks, fixed safe GDB serial argv and non-opening file/device checks. A confirmed
launch helper revalidates then execs GDB; no configuration, SysRq, serial break,
rebuild/deploy/reset or privilege changes. Add normal/failure/fake-process tests.
Run the registry's focused kgdb_serial checks, formatting, affected strict Clippy
and roadmap, then commit independently with a version bump. Continue immediately
to KGDB-SERIAL-UI-001 and final push/source-bound release. Full suite deferred.
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
