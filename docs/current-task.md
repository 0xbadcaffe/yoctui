# Current Task

**ID:** M67-LIVE-EVIDENCE-001
**Title:** Supply current-source real-Poky release performance evidence
**Status:** BLOCKED

The retained performance evidence is not bound to the current source tree: its
manifest has 143 documented source digest mismatches, including changes predating
M67. Supply a new genuine source/binary-bound Yocto 6.0.2 `linux-yocto` compile
capture using the documented release workload. Do not rewrite historical
digests or substitute fake-process startup timings for live evidence.

```bash
./scripts/verify-performance.sh --real-poky-evidence
./scripts/verify-completion.sh
```

This external prerequisite is the only remaining required task. The user
explicitly deferred the full test suite; do not run the full completion suite
without a subsequent instruction. The v0.1.256 Kernel debugging request passes
focused checks and harmless live GDB terminal smoke. No live target debugging
or source-bound Yocto performance certification is claimed by that smoke.
