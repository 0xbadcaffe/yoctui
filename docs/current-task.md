# Current task

## Active task

**ID:** SDK-MODEL-001
**Title:** Model typed SDK workspace and operations

## Objective

Add the pure typed SDK domain, reducer state/effects, Navigator destination,
populate/test previews, artifact inventory state, publication/native drafts,
shared-job session context, and mechanical app input mapping.

## Required work

1. Inspect `Screen`, Navigator/command mappings, `BuildRequest`, image target
   selection, artifact state patterns, background-job reducers, Wic/QEMU
   session patterns, and app input routing before writing code.
2. Add a pure SDK module with bounded exact machine/distro/image/artifact,
   standard/extensible kind, inventory request/state, publication destination,
   extracted-root, native recipe/tool/argv, preview, session, and outcome
   identities. Validate all tokens and normalized absolute paths without
   filesystem access.
3. Add `Screen::Sdk` as a first-class Navigator destination after Images and
   preserve selection/focus/session compatibility.
4. Model standard `do_populate_sdk`, extensible `do_populate_sdk_ext`,
   `do_testsdk`, and `do_testsdkext` dialogs as exact `BuildRequest` previews
   against authoritative image targets. Confirmation emits the existing
   managed build effect; cancellation is inert.
5. Add generation-correlated SDK artifact inventory, exact selection/search,
   explicit not-loaded/loading/empty/available/partial/failed state, stable
   refresh, and stale-event rejection.
6. Add bounded publication/native-tool drafts and separate indexed exact
   previews. Phrase no command as shell text; use typed paths/tokens/arguments.
   Add shared `BackgroundJobKind::Sdk` session correlation and lifecycle
   actions/effects without duplicating lifecycle storage.
7. Add reducer tests for normal, validation, stale, cancellation/rejection,
   terminal, retention, and navigation paths. Add app tests for SDK workspace
   keys, modal focus, adapter-event normalization, and no shortcut leakage.
8. Update `docs/ui-spec.md` for any intentional behavior change and
   `docs/architecture.md` for any boundary change.
9. Run focused and baseline checks, then hand off to `SDK-ARTIFACT-001`.

## Definition of done

- SDK state and every request/effect are typed, bounded, and filesystem-free.
- Populate/test previews reuse exact existing managed build requests.
- Artifact and SDK-tool state is correlated and stale-safe.
- SDK sessions reuse shared background jobs.
- SDK is reachable through Navigator and app input is modal/typed.
- Focused model/app and all baseline checks pass.

## Verification

```bash
cargo test -p yoctui-model sdk_workflow
cargo test -p yoctui-app sdk_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
