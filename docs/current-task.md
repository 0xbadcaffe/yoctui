# Current Task

**ID:** M67-LIVE-EVIDENCE-001
**Title:** Supply current-source real-Poky release performance evidence
**Status:** BLOCKED

The retained performance evidence is not bound to the current source tree: its
manifest has 143 documented source digest mismatches, including changes predating
M67. Supply a new genuine source/binary-bound Yocto 6.0.2 linux-yocto compile
capture using the documented release workload. Do not rewrite historical
digests or substitute fake-process startup timings for live evidence.

```bash
./scripts/verify-performance.sh --real-poky-evidence
./scripts/verify-completion.sh
```

This external prerequisite is the only remaining required task. The user
explicitly deferred the full suite; do not run the full completion suite without
a subsequent instruction. The v0.1.257 Kernel QEMU → GDB request passes focused
checks and genuine Linux guest/embedded reconnect debugging smokes. That does
not certify current-source Yocto compile performance. All M104 tasks are DONE;
final push and source-bound release delivery are the handoff.
