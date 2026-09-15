# Current Task

**ID:** SEARCH-HELP-001
**Title:** Align search help and README with content-only results
**Status:** DONE

Terminal handoff: all 736 registered tasks are DONE. The requested fresh setup,
daemon startup and global search fixes are implemented. Full workspace tests,
strict Clippy, formatting, 52 bridge tests and roadmap checks pass. The daemon
startup fixture fell from 3.246 seconds to 116 milliseconds with the same
three-second compatibility probe; early attach/update, failure and cancellation
regressions pass. Run `./scripts/verify-completion.sh` for the final release gate.
