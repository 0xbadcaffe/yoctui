# Current Task

## Task

**ID:** UI-VISION-001
**Title:** Complete the dense terminal workbench redesign
**Status:** IN_PROGRESS

## Objective

Run the parent acceptance gate over the complete approved workbench redesign,
confirm typed-state honesty and responsive/accessibility behavior, and prepare
the terminal handoff without masking the independent host perf-policy blocker.

## Dependencies

- `UI-VISION-SHELL-001` — DONE
- `UI-VISION-NAV-001` — DONE
- `UI-VISION-TASKS-001` — DONE
- `UI-VISION-RESP-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `scripts/test-tui-snapshots.sh`
- `docs/ui-spec.md`
- `docs/task-registry.toml`
- `docs/implementation-status.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- The approved header, grouped Navigator, task/log/history cockpit, Inspector, and command rail ship together.
- All displayed operational values remain typed and unavailable fields stay explicit.
- Focus, keyboard, mouse, themes, no-color, and every responsive breakpoint remain intact.
- Aggregate project verification passes except the separately documented host perf policy gate.

## Verification

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```
