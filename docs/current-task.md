# Current task

## Active task

**ID:** WIC-WRITE-CLI-001
**Title:** Integrate protected Wic device execution in the CLI

## Objective

Connect the completed protected-device model, rendering, and adapter to the
real CLI event loop so discovery and writes run asynchronously through typed,
generation-correlated state without blocking the terminal.

## Required work

1. Inspect the existing Wic creation capability/runner integration, Images
   dialog input routing, shared operation polling, and protected device adapter
   APIs before editing.
2. Execute `Effect::GetWicDevices` through the adapter without blocking the UI,
   preserve its generation and exact image identity, and normalize success,
   partial limitations, timeout, failure, and stale completion into typed model
   actions.
3. Route picker, phrase-entry, exact-preview, and stronger write-cancellation
   inputs before global Images shortcuts so modal focus cannot leak.
4. Execute `Effect::StartWicSession` for writes through the existing managed Wic
   runner. Independently reconstruct and adapter-revalidate exact image/device
   identity and argv immediately before spawn; never execute a widget-derived
   command or shell string.
5. Poll write output and lifecycle nonblockingly through the shared managed-job
   path. Preserve stream tags, truncation/drop counts, host telemetry, success,
   nonzero failure, graceful/forced cancellation, rejection, and unexpected
   process loss across navigation.
6. Keep creation and writing mutually exclusive while preserving both typed
   operation histories. A write cancellation must acknowledge the
   incomplete-device warning; ordinary creation cancellation must not inherit
   that acknowledgement.
7. Add focused fake-device/process CLI tests named `wic_device_write` covering
   discovery success/partial/empty/failure, modal routing, exact revalidation,
   success, nonzero failure, cancellation escalation, rejection, stale
   completion, and loss. Do not claim live removable-media safety.
8. Run focused and baseline checks, then hand off to
   `WIC-WRITE-UI-CLI-001`.

## Definition of done

- `D` drives real asynchronous adapter discovery in the CLI.
- Confirmed previews execute only after independent adapter revalidation.
- Every discovery and runner outcome reaches correlated typed model state.
- Device-write cancellation retains its stronger acknowledgement semantics.
- Focused fake integrations and all baseline checks pass.

## Verification

```bash
cargo test -p yoctui -- wic_device_write
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
