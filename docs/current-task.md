# Current Task

**ID:** EDITOR-GITUI-CONTEXT-001
**Title:** Open GitUI at the active integrated editor repository
**Status:** IN_PROGRESS

MODIFIER-SHORTCUTS-001 is DONE in v0.1.261 with focused app/model/UI/CLI
verification, strict Clippy, formatting, version policy and roadmap passing.
Enable GitUI for all integrated editor contexts with exact-root bounded
asynchronous read-only inspection, correlated results, retained dirty state and
contextual F12 routing. Check missing tool/nonrepository/invalid/stale failure
paths and existing Devtool/terminal flows. Run the registry's focused editor
GitUI checks, formatting, strict Clippy, version policy and roadmap. Bump version,
build optimized release, commit, push and bind the final binary to that commit.
Do not run the full test suite.

The separate external blocker remains:

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
