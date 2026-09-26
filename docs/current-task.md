# Current Task

**ID:** PTY-SESSION-ID-RECOVERY-001
**Title:** Allocate new PTY identities above recovered terminal history
**Status:** IN_PROGRESS

A live Romulus launch opened ncurses successfully in the daemon PTY but left
the platform workspace at `Starting Kernel menuconfig`. The restarted daemon
allocated session ID 1 even though recovered lost sessions already contained
that ID. Seed new PTY allocation above all recovered terminal identities so
the client can distinguish and bind the newly requested session.

Verify with:

```bash
cargo test -p yoctui --bin yoctui recovered_pty
cargo test -p yoctui-model platform_menuconfig
cargo fmt --all --check
```
