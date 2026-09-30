# Current Task

**ID:** M67-LIVE-EVIDENCE-001
**Title:** Supply current-source real-Poky release performance evidence
**Status:** BLOCKED

The retained performance evidence is not bound to the current source tree: its
manifest has 143 source digest mismatches, including changes predating M67.
Supply a new genuine source/binary-bound Yocto 6.0.2 `linux-yocto` compile
capture using the documented release workload. Do not rewrite historical
digests or substitute fake-process startup timings for live evidence.

```bash
./scripts/verify-performance.sh --real-poky-evidence
./scripts/verify-completion.sh
```

This external validation prerequisite is the only remaining required task.
The user explicitly deferred the full test suite for the v0.1.253 Files change;
do not run the full completion suite without a subsequent instruction.
