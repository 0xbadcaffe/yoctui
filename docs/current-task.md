# Current Task

## Task

**ID:** MAINT-SERVICE-ADAPTER-001
**Title:** Adapt PR and hash service diagnostics

## Objective

Inspect authoritative PR service and hash server configuration plus bounded
observational process evidence, and expose only the installed documented
`bitbake-prserv-tool` export/import operations. Yoctui must never launch,
restart, stop, or reconfigure internal BitBake services.

## Required work

1. Accept an explicit initialized metadata snapshot containing `PRSERV_HOST`,
   `BB_HASHSERVE`, `BB_HASHSERVE_UPSTREAM`, signature configuration, build
   identity, and a child-only executable search path.
2. Normalize configured, disabled, local, remote, reachable, unreachable,
   partial, and unavailable PR/hash states without treating process-name
   matching as proof of endpoint health.
3. Acquire bounded observational evidence for `bitbake-prserv`,
   `bitbake-hashserv`, and `bitbake-worker`; never own their lifecycle.
4. Discover only a canonical regular non-symlink `bitbake-prserv-tool` and
   expose only documented export and import operations.
5. Construct exact shell-free vectors for canonical writable `.conf`/`.inc`
   export destinations and canonical readable regular `.conf`/`.inc` import
   sources. Revalidate executable and file identities immediately before
   execution.
6. Preserve the helper's known memory-server and BitBake-cache side effects in
   typed previews; do not infer or expose undocumented commands.
7. Reuse one Maintenance process runner where practical, with fake-process
   tests for exact diagnostics and vectors, missing/unsafe/tampered inputs,
   success, nonzero failure, timeout, graceful/forced cancellation, rejection,
   and runner loss.
8. Do not claim live PR/hash service health or PR database compatibility from
   fixture tests.

## Definition of done

- Service configuration, endpoint observations, and process evidence remain
  typed, bounded, and explicit about limitations.
- Process evidence is observational only and cannot mutate service lifecycle.
- Only exact documented PR export/import vectors can execute.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake maintenance_service
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/architecture.md` only if the adapter boundary changes.
- Mark `MAINT-SERVICE-ADAPTER-001` `DONE` only after verification passes.
- Update `docs/implementation-status.md`.
- Replace this file with the next eligible highest-priority Maintenance task.

## Next task

`MAINT-RELEASE-ADAPTER-001`
