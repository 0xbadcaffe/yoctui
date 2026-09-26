# Current Task

**ID:** HARDWARE-MODEL-001
**Title:** Add typed Hardware library and viewer state
**Status:** IN_PROGRESS

Add the closed Hardware categories and supported document kinds, bounded
library/browser/viewer/search state, typed reducer actions/effects, and normal
and failure-path unit tests. Do not perform filesystem or converter work in the
model.

Verify with:

```bash
cargo test -p yoctui-model hardware
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
