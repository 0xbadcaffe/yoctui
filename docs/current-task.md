# Current task

## Active task

**ID:** SDK-CLI-001
**Title:** Integrate SDK execution in the CLI

## Objective

Connect the typed SDK effects to independently polled artifact scans, tool
capability inspection, managed BitBake populate/test builds, and SDK
publication/native runners while keeping terminal input, rendering, telemetry,
and unrelated jobs responsive.

## Required work

1. Inspect the completed SDK model/effects, artifact and tool adapters, app
   mappings, and existing image/package/signature/QEMU/Wic CLI coordinators
   before writing code.
2. Construct SDK adapters only from the active canonical build directory,
   typed `SDK_DEPLOY`, and authoritative source/workspace roots. Route
   `InspectSdkTools` into the typed capability state with individual missing
   tools preserved.
3. Own at most one replaceable generation-correlated SDK artifact scan and
   cancellation token. Poll it independently; map empty/complete/partial,
   invalid configuration, timeout, cancellation, and worker loss into the
   existing typed reducer actions. Stale results must remain reducer-inert.
4. Route SDK populate/test `BuildRequest` values through the existing managed
   BitBake coordinator without introducing a parallel build lifecycle. Refresh
   the exact SDK inventory only after a successful populate operation.
5. Reconstruct publication/native `SdkToolCommandSpec` values through the
   adapter immediately before spawn. Own one `SdkToolJobRunner`, poll typed
   started/output/success/nonzero/timeout/loss events, and map them to the exact
   `SdkSessionId` reducer actions. Refresh the inventory after successful
   publication.
6. Route cancellation to the SDK runner independently from BitBake, QEMU, Wic,
   package, signature, and artifact-scan cancellation. Preserve rejection,
   graceful/forced cancellation, exit codes, retained output, and navigation.
7. Add focused CLI tests named `sdk_workflow` with fake filesystem/process
   adapters covering capability, loading/empty/partial/failure scan outcomes,
   replacement/stale results, populate/test reuse, publication/native
   execution, child-only environment, output, success/nonzero/timeout,
   graceful/forced cancellation, rejection, startup/runner loss, refresh, and
   simultaneous navigation/telemetry polling. Do not claim live SDK support.
8. Update architecture only if ownership differs from the Managed SDK
   boundary. Run focused and baseline checks, then hand off to
   `SDK-UI-CLI-001`.

## Definition of done

- Every SDK effect has a nonblocking CLI execution route with exact typed
  correlation and no widget/backend state mutation.
- SDK builds reuse the existing BitBake job lifecycle; scans and SDK tools are
  independently cancellable and polled.
- Success, failure, timeout, cancellation/rejection, and loss remain distinct,
  durable, and navigation-safe.
- Focused fake integration and baseline verification pass.

## Verification

```bash
cargo test -p yoctui -- sdk_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
