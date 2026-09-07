# Compact telemetry regression — v0.1.65 candidate

## Cause and change

Dashboard and Tasks previously removed telemetry below their full-height layout.
The reported 181x43 terminal therefore hid CPU, RAM and build-filesystem meters.
The new fallback reserves four rows for the three typed percentages and
square-dot bars. Unknown readings remain unavailable; zero remains a valid
measurement. It adds no sampling, animation, focus target or backend operation.

Rendering and mouse task selection now share panel heights. Mouse hit testing
also accounts for Dashboard's four summary rows rather than Tasks' two, so
clicks on logs, history, borders or resources cannot select hidden task rows.

## Regression evidence

The initial `compact_resource_meters_remain_visible_across_workspace_sizes`
test failed at 181x43 with `missing Resources` before implementation.
After the fix, both Dashboard and Tasks pass at 181x43, 160x48, 130x40, 100x30
and 80x24. Coverage includes selection/focus retention, visible compile task
and Log Viewer, full-size telemetry preservation, missing/invalid readings,
true zero, ASCII/no-color/reduced-motion, tiny widget bounds and mouse rows.

Completed checks on 2026-09-07:

- `cargo test -p yoctui-ui -p yoctui-app --lib`: 468 passed.
- `cargo test --workspace --all-features`: 1,520 passed, 4 existing ignored.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all --check`: passed.
- `python3 -m pytest bridge/tests`: 46 passed.

The first workspace compilation failed with ENOSPC. The full retry passed
after cleaning only prior agent-generated address/leak sanitizer caches.
Local logs: `/tmp/yoctui-v65-ui-tests.log`,
`/tmp/yoctui-v65-workspace-tests-retry.log`,
`/tmp/yoctui-v65-clippy.log`, `/tmp/yoctui-v65-bridge-tests.log`.

## Visual review

Six concept captures and four 160x50 target cell fixtures change only the
version digit. Their six production PNGs were regenerated with the existing
deterministic raster script. At 160x48, the intentional reviewed change keeps
the 17-row task table and nine-row history, shortens logs from 18 to 14 rows,
and adds Resources at rows 42–45. Inspector and command-rail geometry remain
unchanged. The literal fixture shows unavailable RAM/FS because those fixture
values are absent; it must not invent readings to resemble a concept image.

## Delivery boundary

These are candidate code/test results, not an installation or crates.io release.
Installed Yoctui remains v0.1.64. The old v0.1.21 daemon had stale negative
pkgdata authority although 4,387 generated packages were directly readable.
At the user's request, its job was cancelled and daemon stopped; no production
Poky build was restarted. A fresh daemon authority check remains necessary.

The tested debug candidate was preserved at
`/tmp/yoctui-v65-validated.AoS2PA/yoctui`, SHA-256
`b6620a43e5b725ffa0ad5a916d943bc075e5d4d94181ed540ec2be7ab4ec13c5`,
before cleaning Cargo workspace debug outputs (79.1 GiB) to make room for
OpenBMC. This is not the optimized release-performance candidate.

Full completion and source-bound real-Poky performance validation are separate
gates; v0.1.64's recorded live metrics do not validate this candidate's changed
runtime sources. `./scripts/verify-performance.sh --real-poky-evidence` passed
the deterministic checks through coexistence, then correctly rejected the old
record with `real-Poky source digest mismatch: crates/yoctui-app/src/lib.rs`.
The record was not rewritten or relabeled as new evidence. OpenBMC validation
also remains pending the documented
[storage dependency](openbmc.md#storage-dependency).
