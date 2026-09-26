# Current Task

**ID:** PLATFORM-INSPECTION-ENV-001
**Title:** Initialize the selected build environment for platform inspection
**Status:** IN_PROGRESS

Reproduce from a plain terminal while the initialized Romulus daemon is
running: restoring Kernel or U-Boot begins inspection, then the client-created
bridge fails because `bb` is absent from its inherited Python path. Initialize
the selected build profile off the terminal loop and provide its exact
environment to the capability-authorized bridge for both platform workspaces.

Verify with:

```bash
cargo test -p yoctui --bin yoctui platform_inspection
cargo test -p yoctui --bin yoctui startup_environment
cargo fmt --all --check
```
