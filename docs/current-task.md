# Current Task

## Task

**ID:** STATE-COHERENCE-001
**Title:** Complete live client state coherence
**Status:** DONE

## Objective

A long-running attached Yoctui client must not retain active task rows after it
loses daemon authority, and a terminal resize must not leave duplicate
Navigator rows or workspace titles.

## Dependencies

- STATE-RECONNECT-001 — DONE
- UX-REDRAW-001 — DONE

## Definition of done

- Daemon transport loss retires nonterminal task rows as Lost and changes the
  build from Parsing/Running to Lost.
- The client retries a bounded attach and installs the current daemon snapshot.
- Resize invalidates the complete terminal buffer before the next frame.
- Focused state and render regressions cover the photographed failure mode.
- Version 0.1.5 is installed and repository completion gates pass.

## Verification

```bash
cargo test -p yoctui-model build_authority_loss
cargo test -p yoctui terminal_resize_requires_full_redraw
cargo test -p yoctui-ui navigator_and_tasks_titles_render_once_after_resize
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```
