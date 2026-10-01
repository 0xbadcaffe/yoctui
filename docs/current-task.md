# Current Task

**ID:** QEMU-GDB-LIVE-001
**Title:** Verify real Linux guest breakpoint backtrace and resume
**Status:** IN_PROGRESS

Dependency QEMU-GDB-UI-001 is DONE. Record genuine Linux QEMU/GDB breakpoint,
backtrace, resume, interrupt and exit/cleanup evidence with exact artifacts/tool
versions. Validate embedded launch and detach/reconnect without restarting the
user daemon or modifying build configuration. Record limitations honestly.
Update roadmap/status/registry and return current-task to the preserved M67
external blocker when no eligible tasks remain. Push and build final HEAD.

```bash
./scripts/verify-roadmap.sh
```

Manual: real matching Linux boot kernel/vmlinux/rootfs; managed GDB attach,
break start_kernel, bt, continue to login, Ctrl+C, quit; owned PIDs gone and
source/configuration hashes unchanged. Do not substitute fake/host ELF evidence.
