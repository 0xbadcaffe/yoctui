# Current Task

## Task

**ID:** COMPAT-001
**Title:** Complete Yocto release correlated functionality
**Status:** IN_PROGRESS

## Objective

Close the M18 parent gate only after the centralized daemon-owned capability
architecture, dynamic UI/command authority, deterministic compatibility tests,
current latest-plus-older live evidence, and completion verification all pass
together.

## Dependencies

- Every required `COMPAT-*` child — DONE

## Relevant files

- `scripts/verify-compatibility.sh`
- `scripts/verify-completion.sh`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Every required compatibility child remains DONE.
- The dedicated verifier independently checks deterministic model, probe,
  command, UI, future-release, documentation, and current non-fixture live
  evidence gates.
- Completion cannot pass from registry status alone or from fixture evidence.
- The full workspace baseline and roadmap checks pass.
- M18 is promoted to DONE only after all gates pass.

## Verification

```bash
./scripts/verify-compatibility.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```
