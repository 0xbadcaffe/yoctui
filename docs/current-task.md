# Current Task

## Task

**ID:** CONSOLE-TERM-002
**Title:** Verify SSH and runqemu use the admitted tui-term terminal path
**Status:** IN_PROGRESS

## Objective

Verify the existing image-console launch, daemon-owned PTY and tui-term replica
rendering paths together. Preserve exact argv, SSH host-key policy, writer
leases, input, resize, lifecycle, and reconnect. Add regression coverage for
both console kinds without claiming fixture results as real target execution.

## Dependencies

- IMAGE-UDEV-001 — DONE (v0.1.58)

## Definition of done

- SSH/QEMU launches and renderer routes are covered by focused tests.
- Real PTY integration and terminal lifecycle tests pass.
- Documentation explains the shared tui-term path and its authority boundaries.

## Verification

```bash
cargo test --workspace image_console
cargo test -p yoctui --test daemon_pty_runtime
./scripts/test-terminal.sh
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```

Next: YOCTO-LOGGER-ADAPTER-001, LOG-CONSOLE-IMAGE-001.
