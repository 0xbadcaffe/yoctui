# Current Task

**ID:** HARDWARE-UI-001
**Title:** Render and operate the Hardware workspace and full-body viewer
**Status:** IN_PROGRESS

Add Navigator and application-menu routes, categorized library and browser UI,
maximum-body embedded viewer, page/zoom/pan/search controls, Help text, and
responsive TestBackend coverage. Route typed effects to the background worker
and persist successful library mutations immediately.

Verify with:

```bash
cargo test -p yoctui-app hardware
cargo test -p yoctui-ui hardware
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
