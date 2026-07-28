# Current task

## Active task

**ID:** WIC-MODEL-001
**Title:** Add typed Wic creation and device-write state

## Objective

Add the pure typed identities, bounded inventories, deterministic previews,
shared-job lifecycle, and destructive confirmation state used by the Wic
creation and device-write children.

## Required work

1. Inspect the image, QEMU, and background-job model/reducer implementation
   before writing code; reuse their lifecycle and correlation behavior.
2. Add a dedicated pure `wic` model module for canonical tool, image,
   kickstart, generated-output, and block-device identities.
3. Add bounded typed capability, kickstart source/partition preview, output
   inventory, and device inventory states with deterministic normalization and
   explicit partial/failure/unavailable distinctions.
4. Add exact cooked-mode creation and device-write requests, drafts, and
   deterministic argument previews. Validate normalized absolute paths, typed
   options, exact inventory membership, output bounds, and
   `WRITE <canonical-device-path>` without filesystem access.
5. Add stable Wic creation/write sessions backed by disjoint shared background
   jobs, bounded stream-tagged output, stale-event rejection, all terminal
   outcomes, cancellation confirmation, and a second write-cancellation
   warning.
6. Add reducer actions/effects and mechanical app normalization for capability,
   inventories, start/cancel, output, and terminal events. Do not parse raw
   adapter/process text in the app.
7. Cover normal, invalid, stale, bounded, destructive-confirmation, and
   lifecycle paths with `wic_model` tests.
8. Update architecture only if implementation changes the reconciled boundary,
   then mark the child done and hand off to `WIC-ADAPTER-001`.

## Definition of done

- All Wic domain/reducer state is pure, bounded, and identity-correlated.
- Creation and write previews are deterministic and shell-free.
- Device-write state cannot emit a start effect without exact typed phrase and
  confirmation gates.
- Shared jobs retain all lifecycle/output/terminal history without colliding
  with BitBake, Devtool, or QEMU IDs.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-model wic_model
cargo test -p yoctui-app wic_model
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
