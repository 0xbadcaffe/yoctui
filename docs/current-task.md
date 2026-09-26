# Current Task

**ID:** DETACHED-TERMINAL-STARTUP-001
**Title:** Verify detached menuconfig terminal startup
**Status:** IN_PROGRESS

Prefer explicitly supported terminal emulators, encode their foreground/wait
arguments without a shell, and reject launchers that exit during a bounded
startup probe. A successful notification must mean the terminal launcher is
still alive rather than merely that `spawn` returned.

Verification:

```bash
cargo test -p yoctui --bin yoctui detached_terminal
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
