# Current task

## Active task

**ID:** SEC-ADAPTER-001
**Title:** Adapt CVE and SPDX metadata and reports

## Objective

Close the Security adapter parent gate across capability inspection, bounded
report acquisition/parsing, and exact package mapping.

## Required work

1. Inspect the capability, report, and mapper adapter implementations together
   for contract gaps, duplicate behavior, or inconsistent public exports.
2. Verify all adapter inputs derive only from typed authoritative scope,
   canonical paths, exact artifact identities, or explicit imports.
3. Verify every filesystem/process boundary is bounded, fail-closed,
   cancellable where applicable, and emits only typed data/events.
4. Verify current and legacy task selection remains capability-supplied and no
   release-name, filename, raw log, or display-text inference was introduced.
5. Run the complete focused Security adapter and app gates plus the baseline.
6. Fix any discovered cross-child inconsistency without broadening this parent
   task into rendering or CLI integration.
7. Record that fake process/filesystem coverage does not establish live Yocto
   Security compatibility.

## Definition of done

- Capability, report, and mapper children form one consistent typed adapter
  boundary.
- Focused `security` and app gates pass without weakening tests.
- No mocked fixture is presented as live BitBake/Yocto support.
- Registry/status/current-task governance advances to the next eligible task.

## Verification

```bash
cargo test -p yoctui-bitbake security
cargo test -p yoctui-app security_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
