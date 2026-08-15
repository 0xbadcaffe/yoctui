# Current Task

## Task

**ID:** BRIDGE-PROGRESS-001
**Title:** Normalize live BitBake progress and render task bars
**Status:** IN_PROGRESS

## Objective

Keep live Poky builds connected when BitBake emits fractional process progress,
correlate PID-only task-progress events with their authoritative task-start
identity, and render determinate per-task progress bars without fabricating
progress when BitBake reports an unknown or invalid value.

The 2026-08-15 live Scarthgap `core-image-minimal` build reproduced both gaps:
`ProcessProgress.progress` emitted `77.92379445665797`, which crossed the bridge
unchanged and violated the protocol's `u64` parse-progress field, while
`bb.build.TaskProgress` correctly carried only its worker PID and was reduced to
an unrecognized-event warning because the bridge expected recipe/task strings.
The live BitBake server and workers remain authoritative and are not to be
terminated while this compatibility fix is developed.

## Relevant files

- `bridge/yoctui_bridge.py`
- `bridge/tests/test_bridge.py`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/product-roadmap.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Fractional finite process progress is bounded and converted to an integer at
  the bridge boundary; negative, non-finite, boolean, and malformed values
  remain unknown instead of violating the wire schema.
- Task-start identities are retained by valid worker PID only for the active
  build and PID-only task-progress events reuse that identity.
- Stale or identity-less task progress is ignored safely rather than producing
  warning floods or fabricated task identities.
- Dashboard and Tasks views render determinate per-task bars when progress is
  available and preserve explicit animated/unknown behavior otherwise.
- Mocked native-event tests cover fractional process progress, PID correlation,
  invalid progress, and completion cleanup.
- Focused verification, baseline checks, and a non-disruptive live reconnect
  against the running build pass.
- Registry and human-readable status return to `DONE`, and the change is
  committed as one coherent implementation task.

## Verification

```bash
python3 -m pytest bridge/tests
cargo test -p yoctui-ui task_progress
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
