# Current Task

## Task

**ID:** UI-WIDE-RAIL-001
**Title:** Keep the reference F-key rail visible on wide screens
**Status:** IN_PROGRESS

## Objective

Make the stable reference F1–F10 navigation rail visible on every wide
workbench screen instead of only at exactly 160 columns on Tasks.

## Dependencies

- `UI-LITERAL-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `artifacts/release-quality/snapshots/`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Dashboard and Tasks show every F1–F10 label at widths 130, 160, 180, and 200.
- The exact canonical 160×48 cell/style golden remains unchanged.
- Widths below 130 retain contextual shortcuts and remain narrow-safe.
- Real PTY snapshots and the installed release artifact are refreshed and verified.

## Verification

```bash
cargo test -p yoctui-ui wide_reference_rail
cargo test -p yoctui-ui literal_reference_cell_and_style_golden
./scripts/test-tui-snapshots.sh
cargo fmt --all --check
cargo clippy -p yoctui-ui --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
