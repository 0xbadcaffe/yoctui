# Current Task

## Task

**ID:** UX-BRAILLE-ACTIVITY-001
**Title:** Standardize animated activity on Braille Eight Double
**Status:** IN_PROGRESS

## Objective

Every animated Unicode activity indicator uses the reviewed eight-frame
`throbber-widgets-tui::BRAILLE_EIGHT_DOUBLE` set from one shared renderer.

## Dependencies

- UX-VIEWPORT-CUES-001 — DONE

## Definition of done

- The shared activity adapter selects `BRAILLE_EIGHT_DOUBLE` for Unicode.
- All active task/progress call sites continue through that adapter.
- ASCII animation remains `|/-\\` and reduced motion remains stable text.
- Terminal success, failure, and cancellation markers never animate.
- Focused, workspace, Clippy, documentation, and roadmap checks pass.

## Verification

```bash
cargo test -p yoctui-ui ux_throbber
cargo test -p yoctui-ui active_task_indicator
cargo test -p yoctui-model ux_throbber
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```

Activity phase remains reducer-owned. The UI maps that immutable phase to an
admitted symbol constant and retains no third-party widget state.
