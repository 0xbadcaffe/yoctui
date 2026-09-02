# Current Task

## Task

**ID:** IMAGE-CONSOLE-001
**Title:** Complete QEMU and SSH image consoles
**Status:** DONE

## Objective

Images exposes one typed, bounded console request that either boots the exact
selected artifact with `runqemu` in a daemon-owned PTY or connects to an
explicit SSH destination in the same terminal workbench.

## Dependencies

- UX-TERMINAL-UX-001 — DONE
- IMAGE-TARGET-AUTH-001 — DONE

## Definition of done

- QEMU console launch is bound to the selected artifact and current inspected
  `runqemu` executable, with `nographic` and `serialstdio` enforced.
- SSH requires an explicit validated host, user, port, and optional normalized
  absolute identity path; it retains normal host-key verification, persists no
  password, and supplies no remote shell command.
- Both modes emit argv-preserving daemon PTY requests and use the existing
  `tui-term` replica renderer.
- Opening or cancelling the dialog spawns nothing; unavailable executables and
  stale image authority fail closed with exact reasons.
- Focused model, app, protocol, daemon, UI, workspace, Clippy, documentation,
  roadmap, and completion checks pass.

## Verification

```bash
cargo test -p yoctui-model image_console
cargo test -p yoctui-app image_console
cargo test -p yoctui-protocol image_console
cargo test -p yoctui-cli image_console
cargo test -p yoctui-ui image_console
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

SSH is a connection to an already-running target, not a claim that SSH boots
the image. QEMU is the boot path. Both remain daemon-owned terminal sessions.
