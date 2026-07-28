# Current task

## Active task

**ID:** WIC-WRITE-UI-CLI-001
**Title:** Integrate protected Wic device writing

## Objective

Connect the completed fail-closed device adapter to the typed Wic model,
responsive Images UI, and independent CLI coordination so a user can select an
eligible device, enter the exact destructive phrase, preview and run the write,
and cancel only through the stronger incomplete-device warning.

## Required work

1. Inspect the existing Wic device inventory/write reducer state, shared dialog
   queue, Images rendering/input, creation CLI coordinator, and adapter response
   seams before editing; do not duplicate capability or job state.
2. Add reducer effects and transitions that start generation-correlated device
   discovery only for an exact selected uncompressed `.wic` or `.direct`
   generated/deployed image. Preserve exact image selection and reject stale
   inventory responses.
3. Implement a focus-trapping device picker with typed whole-device summaries,
   explicit empty/partial/failure states, bounded selection, and every adapter
   limitation. Selection must not authorize a write.
4. Implement the exact `WRITE <canonical-device-path>` phrase entry and a
   separate exact shell-free command preview. Revalidate current capability,
   image, device, and phrase state before emitting the write effect.
5. Start a distinct persistent Wic write session through the completed adapter
   and runner. Poll it independently of BitBake, Devtool, QEMU, Wic capability,
   and creation work while retaining exact image/device identity and bounded
   output across navigation.
6. Route write cancellation through the second device-incomplete warning.
   Preserve distinct success, nonzero failure, graceful/forced cancellation,
   cancellation rejection, and process-loss outcomes.
7. Render device capability, picker, phrase, preview, running telemetry/output,
   and terminal history responsively at every supported breakpoint and in every
   semantic theme. Keep the documented `D`, `W`, `w`, `x`, generated-output,
   and artifact shortcuts unambiguous.
8. Add reducer, app-input, Ratatui `TestBackend`, and fake CLI/runner tests for
   normal, empty, partial, failed, stale, wrong-phrase, cancellation, terminal,
   narrow, long-content, and navigation-retention paths. Do not claim live
   removable-media safety from fake paths.
9. Run focused and baseline checks, then mark the child done and hand off to the
   `WIC-001` parent gate.

## Definition of done

- Only an exact eligible Wic image can open generation-correlated discovery.
- The picker, exact phrase, and separate command preview all trap focus and
  cannot be bypassed by global or workspace input.
- The CLI independently executes and polls only adapter-revalidated writes.
- Every terminal and cancellation outcome retains exact image/device context.
- Responsive reducer, input, TestBackend, and fake-process checks pass without
  making a live hardware claim.

## Verification

```bash
cargo test -p yoctui-model wic_device_write
cargo test -p yoctui-app wic_device_write
cargo test -p yoctui-ui wic_device_write
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
