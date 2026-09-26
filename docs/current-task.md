# Current Task

**ID:** MENUCONFIG-RELAY-ISOLATION-001
**Title:** Isolate concurrent Kernel and U-Boot menuconfig relays
**Status:** IN_PROGRESS

The fixed `menuconfig.sock` permits a later relay's liveness connection to be
accepted as an earlier relay's one-shot command, producing EOF/broken-pipe task
failures. Allocate a unique private socket for each relay process and ensure
retries and concurrent Kernel/U-Boot sessions cannot touch one another.

Verify with:

```bash
cargo test -p yoctui --bin yoctui menuconfig_relay
cargo fmt --all --check
```
