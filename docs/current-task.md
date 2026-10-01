# Current Task

**ID:** MODIFIER-SHORTCUTS-001
**Title:** Make application shortcuts portable modifier combinations
**Status:** IN_PROGRESS

User priority supersedes the blocked performance queue. Add typed Alt-letter
input and keymap support, modifier alternatives for uppercase application
commands, consistent workspace-open/GitUI routes, and Alt+f workspace search.
Preserve literal text, modal focus, user keymaps and native PTY keys. Verify
focused modifier/keymap/Devtool/search/UI/CLI checks, formatting, affected strict
Clippy, version policy and roadmap; bump version and commit. Then complete
EDITOR-GITUI-CONTEXT-001 and push/build the final source-bound release.
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
