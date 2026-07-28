# Current task

## Active task

**ID:** WIC-ADAPTER-001
**Title:** Verify the complete Wic creation adapter

## Objective

Verify canonical capability/kickstart/command construction and managed
creation/output scanning as one coherent adapter boundary.

## Required work

1. Inspect both completed adapter children and their app normalization tests.
2. Run every focused verification command for the parent.
3. Confirm capability identity, parsed preview, command revalidation, process
   lifecycle, persistent timeout, and output snapshot identities agree.
4. Confirm symlinks, malformed/oversized inputs, duplicates, nonzero exits,
   cancellation/rejection, and process loss remain explicit.
5. Confirm fake evidence is not presented as live Wic compatibility; the active
   BitBake-only source lacks the required Wic installation.
6. Run all baseline checks. Mark the parent done and hand off to
   `WIC-UI-MODEL-001` only when every check passes.

## Definition of done

- Both focused adapter/app parent checks pass.
- Capability through terminal output remains typed, bounded, and shell-free.
- Only exact new canonical files under the requested root are returned.
- Fake coverage is not presented as live compatibility.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake wic_adapter
cargo test -p yoctui-app wic_adapter
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
