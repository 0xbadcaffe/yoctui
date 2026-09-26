# Current Task

**ID:** MENUCONFIG-PTY-ISOLATION-001
**Title:** Give embedded menuconfig exclusive PTY streams
**Status:** IN_PROGRESS

Live Romulus evidence shows BitBake's Knotty footer overwriting the ncurses
screen and sharing input while `do_menuconfig` waits at 99%. Isolate the outer
BitBake client's standard streams so only the validated menuconfig wrapper
owns the selected PTY. Preserve argv, private handoff sockets, validation, and
terminal outcome propagation.

```bash
cargo test -p yoctui --bin yoctui menuconfig_relay
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
