# Current Task

## Task

**ID:** BRIDGE-PROGRESS-001
**Title:** Normalize live BitBake progress and render task bars
**Status:** DONE

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
The original client and build had exited before the corrected binary was
installed, so final live validation used read-only inspection of the same Poky
build directory and did not claim that a build remained active.

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
- Focused verification, baseline checks, and non-disruptive live bridge
  inspection against the affected Poky build directory pass.
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

The installed binary connected to `/home/bspguy-dev/src/poky/build-yoctui`
through BitBake 2.8.1 and reported the Scarthgap 5.0.19 workspace without a JSON
protocol error or reconnect loop. Bridge coverage is 78%, all 39 bridge tests
pass, both focused task-progress UI tests pass, and the workspace tests, Clippy,
documentation, and roadmap checks pass.
