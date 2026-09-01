# Current Task

## Task

**ID:** UX-EXTERNAL-REDRAW-001
**Title:** Invalidate retained terminal cells after external processes
**Status:** DONE

## Objective

Returning from Neovim, another external editor, or an inherited Yocto shell
must restore a clean Yoctui frame without retained colors or glyphs.

## Dependencies

- PKGDATA-AUTH-SYNC-001 — DONE

## Definition of done

- Every successful terminal resume requests a full redraw.
- Ratatui's retained cell buffer and the physical terminal are cleared before
  the next frame.
- One resume causes exactly one clear; normal frames do not flicker.
- External-editor failure paths still restore and repaint the workbench.
- Version 0.1.8 is installed and repository completion gates pass.

## Verification

```bash
cargo test -p yoctui external_process_redraw_latch_is_edge_triggered
cargo test -p yoctui pkgdata_workspace_background_operation_binds_current_authority_and_reports_results
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

The reported Neovim bleed-through was caused by re-entering the alternate
screen while Ratatui still retained the pre-editor frame. The resume path now
sets an edge-triggered invalidation latch, and the event loop consumes it by
clearing the backend immediately before rendering the restored frame.
