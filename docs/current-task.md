# Current Task

## Task

**ID:** UX-INT-001
**Title:** Validate dedicated onboarding UX end to end
**Status:** IN_PROGRESS

## Objective

Validate startup focus, profile replacement, setup shell, typed image availability,
theme rendering, and all terminal outcomes.

## Verification

```bash
cargo test --workspace --all-features
./scripts/test-tui-pty.sh
./scripts/verify-completion.sh
```

## Definition of done

- Full workspace tests and the PTY onboarding smoke pass.
- Completion gate passes with all registry tasks DONE.

## Next task

Implement `UX-INT-001`.

## Terminal handoff

`UX-THEME-001` completed; `UX-INT-001` is eligible to start.
