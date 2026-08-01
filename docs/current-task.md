# Current Task

## Task

**ID:** MAINT-RELEASE-CLI-001
**Title:** Route Maintenance release forms

## Objective

Connect all three typed Release forms to exact adapter previews and the existing
fresh-inspecting independent Maintenance runner, including local-first archive
creation followed by a separately confirmed optional push.

## Required work

1. Route locked-cache, build-history, and Git-archive preview effects through a
   fresh correlated capability inspection and reject stale identity visibly.
2. Reconstruct only adapter-owned previews with `locked_signature_command`,
   `buildhistory_command`, and `git_archive_local_command`; never spawn before
   generic confirmation.
3. Preserve locked-cache before/after evidence, bounded build-history output,
   local archive HEAD evidence, navigation, cancellation, and every terminal
   outcome through the existing runner.
4. Retain optional archive push intent across the local-only adapter preview.
   Only after successful exact local HEAD capture, perform fresh inspection and
   build `git_archive_push_command`; route it through the existing network
   confirmation and revalidate local HEAD immediately before spawn. Local
   failure/cancel/timeout/loss must never expose or run push.
5. Add fake filesystem/process CLI tests for exact previews, successful changed
   evidence, bounded comparison output, local-only archive, deferred push,
   nonzero failure, stale/input rejection, and changed local HEAD. Do not claim
   live release-tool or network compatibility.

## Definition of done

- All three forms reach exact adapter previews and confirmed independent
  execution after fresh inspection.
- Evidence is installed only after exact successful validation.
- Push remains absent for local-only intent and is offered only after successful
  local evidence with a separate network confirmation.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui -- maintenance_release_workspace
cargo test -p yoctui-bitbake maintenance_release
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/architecture.md` only if coordinator ownership changes.
- Mark `MAINT-RELEASE-CLI-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-UI-CLI-001`.

## Next task

`MAINT-UI-CLI-001`
