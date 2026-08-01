# Current Task

## Task

**ID:** MAINT-OPTIONAL-ADAPTER-001
**Title:** Detect optional release integrations

## Objective

Represent optional pull-request, error-report, repo-manifest, and Toaster
integrations from canonical bounded observations without sending mail,
uploading reports, mutating manifests, or launching services.

## Required work

1. Inspect an explicit initialized build snapshot and child-only search paths
   for canonical regular non-symlink `create-pull-request`,
   `send-pull-request`, `send-error-report`, repo, and Toaster interfaces.
   Preserve missing, partial, unsafe, and unsupported states explicitly.
2. Associate pull-request helpers with one canonical Git worktree and retain
   helper identity separately from repository identity. Do not construct or
   execute mail-sending operations in this detection task.
3. Associate error-report helpers with canonical configured candidates and
   expose only bounded readiness evidence. Do not upload reports or infer
   credentials from helper presence.
4. Detect a canonical repo manifest only when an installed supported interface
   and exact workspace identity are available; otherwise expose an explicit
   unavailable reason. Never create, replace, or mutate a manifest.
5. Detect Toaster executable/configuration capability and bounded observed
   process evidence without starting, stopping, or otherwise managing a
   service. Process observations are diagnostic, not proof of health.
6. Bound search paths, records, strings, and filesystem traversal. Revalidate
   canonical file and directory identities before returning exact evidence.
7. Add fixture tests for complete, missing, partial, unsafe/symlinked,
   tampered, and bounded inputs. Keep every network, mail, upload, manifest,
   and service-lifecycle side effect absent.
8. Do not claim live optional-integration compatibility from fixture tests.

## Definition of done

- Optional integration capabilities are typed, canonical, bounded, and honest
  about partial or unavailable states.
- Detection has no mail, network, manifest-mutation, or service-lifecycle side
  effects.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake maintenance_optional
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/architecture.md` only if the adapter boundary changes.
- Mark `MAINT-OPTIONAL-ADAPTER-001` `DONE` only after verification passes.
- Update `docs/implementation-status.md`.
- Replace this file with the next eligible highest-priority Maintenance task.

## Next task

`MAINT-ADAPTER-001`
