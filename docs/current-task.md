# Current Task

## Task

**ID:** HARDEN-STRESS-001
**Title:** Add deterministic stress and process-tree tests

## Objective

Add repeatable high-volume tests for bounded pure state and protocol framing,
plus a real Unix process-tree cancellation test that proves child descendants
do not survive a runner cancellation.

## Required work

1. Add a deterministic model stress test that drives substantially more log
   events than retention limits and checks count, bytes, loss counters,
   selection, and invariant preservation.
2. Add a deterministic protocol stress test that frames and decodes a large
   ordered stream across irregular chunk boundaries without losing or
   reordering messages.
3. Add a Unix fake-process test using an existing process-group runner. Spawn a
   child that owns a descendant, cancel the exact session, and prove both the
   parent and descendant exit within a bounded deadline.
4. Add `scripts/test-stress.sh` with a validated bounded repetition count and
   focused commands for all three tests.
5. Document scope, reproducible commands, and platform limitations. Do not use
   unbounded loops or timing-only success criteria.

## Definition of done

- Model and protocol high-volume invariants pass deterministically.
- The Unix process-tree test observes and then proves termination of the exact
  descendant after cancellation.
- The bounded repeated stress script passes.
- Focused and baseline verification pass.

## Verification

```bash
./scripts/test-stress.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Document stress and process-tree coverage in `docs/testing.md`.
- Mark `HARDEN-STRESS-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `HARDEN-SANITIZER-001`.

## Next task

`HARDEN-SANITIZER-001`
