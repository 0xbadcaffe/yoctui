# Current Task

## Task

**ID:** DOC-COMPAT-001
**Title:** Complete evidence-backed compatibility matrix

## Objective

Turn the compatibility notes into a structured, auditable matrix that states
exactly what was observed live, what is covered only by fixtures, and which
host or optional-tool prerequisites remain unvalidated or blocked.

## Required work

1. Inspect the live BitBake smoke harness, recorded evidence, bridge adapter
   selection, CLI backend behavior, hardening gates, and current compatibility
   document before editing it.
2. Structure `docs/compatibility.md` by protocol version, host/runtime,
   backend, observed Yocto/BitBake snapshot, capability family, validation
   level, date, and exact evidence command.
3. Keep live, fixture/fake-process, static/test-only, unavailable, and blocked
   evidence visibly distinct. Never generalize one snapshot to all BitBake 2.x
   or claim live support from mocked modules.
4. Record optional tool and host constraints for package data, SDK/QEMU/Wic,
   Testing/Security/QA/Maintenance, terminal and hardening analysis, including
   the current Flamegraph permission blocker.
5. Add a reproducible procedure for adding a supported live combination and
   link installation, operator, testing, and profiling guidance.

## Definition of done

- The matrix identifies the exact observed live combination and capability
  evidence without broad compatibility claims.
- Fixture-only and optional-tool coverage cannot be mistaken for live support.
- Known limitations and blocked evidence include actionable reproduction and
  follow-up commands.
- Task and baseline verification pass.

## Verification

```bash
test -s docs/compatibility.md
python3 scripts/live_bitbake_smoke.py --help
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/compatibility.md` only.
- Mark `DOC-COMPAT-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `DOC-VERIFY-001`.

## Next task

`DOC-VERIFY-001`
