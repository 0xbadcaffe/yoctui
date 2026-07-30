# Current task

## Active task

**ID:** QA-ADAPTER-001
**Title:** Close QA adapter gate

## Objective

Verify the recipe/kernel capability, bounded report, configured-layer runner,
and mechanical app mappings together as one authoritative QA adapter boundary.

## Required work

1. Inspect all three completed QA adapter modules and their app mappings for
   boundary disagreements, duplicated parsing, guessed identities, or missing
   terminal outcomes before changing code.
2. Confirm recipe/kernel tasks remain exact capability values, reports retain
   generation/check/scope/fingerprint identity, and layer execution
   reconstructs only the confirmed canonical native vector.
3. Confirm report and layer workers independently preserve empty, partial,
   malformed, missing, permission, stale, nonzero, cancellation, timeout,
   duplicate, rejection, and loss outcomes where applicable.
4. Add only missing cross-adapter or mechanical app coverage required to close
   the parent gate. Do not implement widgets or CLI polling in this task.
5. Run the focused parent checks and every baseline verification command.

## Definition of done

- All QA adapter and app focused tests pass together.
- No adapter guesses tasks, paths, checks, scopes, formats, status, or native
  arguments outside the documented contracts.
- Raw process/report text cannot cross the app boundary as authority.
- Baseline verification passes.

## Verification

```bash
cargo test -p yoctui-bitbake qa_
cargo test -p yoctui-app qa_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
