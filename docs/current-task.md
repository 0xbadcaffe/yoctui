# Current task

## Active task

**ID:** WIC-WRITE-MODEL-001
**Title:** Add protected Wic device dialog state

## Objective

Complete the pure reducer and app-input workflow that opens discovery from an
exact eligible image, presents an identity-stable device picker, requires the
exact destructive phrase, and separately previews the shell-free write request
before a session can be queued.

## Required work

1. Inspect the existing Wic output/device state, write preview helper, session
   queue, dialog focus helpers, Images actions, and input ordering before
   editing; reuse them rather than creating parallel lifecycle state.
2. Add a typed selected-device identity and focus-trapping dialog variants for
   device inventory/picker, exact phrase entry, and exact command preview.
   Bound selection and phrase input.
3. Add one Images action that resolves only an exact selected generated or
   deployed uncompressed `.wic`/`.direct` identity, advances a non-zero device
   generation, enters loading state, and emits `GetWicDevices`.
4. Correlate loaded/partial/empty/failed results to that exact request. Keep
   stale responses inert and preserve valid selection by full device identity.
5. Route picker movement, selection, phrase editing, preview, cancel/back, and
   final confirmation through pure actions. Selection alone must never emit a
   write effect.
6. Rebuild `WicWritePreview` from current capability, inventory image, exact
   selected device, and exact phrase both before preview and before
   `StartWicSession`. Reject changed output/device/capability state.
7. Open the shared cancellation dialog for an active write with an explicit
   incomplete-device acknowledgement state; ordinary Wic creation cancellation
   remains a one-step confirmation.
8. Add reducer and app-input tests for generated/deployed eligibility, compressed
   or stale rejection, generation correlation, empty/partial/failure states,
   selection stability, bounds, wrong phrase, modal focus/input trapping,
   preview tampering, session queueing, and stronger cancellation.
9. Run focused and baseline checks, then hand off to
   `WIC-WRITE-RENDER-001`.

## Definition of done

- Only an exact uncompressed Wic/direct image can trigger discovery.
- Picker, phrase, preview, and write cancellation are typed modal states.
- Selection and phrase entry cannot bypass the separate exact preview.
- Final confirmation independently reconstructs and compares current state.
- Reducer and app-input focused tests plus the baseline pass.

## Verification

```bash
cargo test -p yoctui-model wic_device_write
cargo test -p yoctui-app wic_device_write
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
