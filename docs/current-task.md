# Current Task

**ID:** M67-LIVE-EVIDENCE-001
**Title:** Supply current-source real-Poky release performance evidence
**Status:** BLOCKED

All M74 corrections, the M75 F12 menu shortcut, and the global-search restore
panic correction are complete through v0.1.201. The only
remaining registry task requires a new genuine source/binary-bound Yocto 6.0.2
`linux-yocto` compile capture. Existing retained evidence is bound to source
base `d2214e82974a5be708a7cc40f1532254d7c7de63` and has 143 source digest
mismatches, including changes predating M67.

After new live evidence is supplied, run:

```bash
./scripts/verify-performance.sh --real-poky-evidence
./scripts/verify-completion.sh
```

Do not rewrite evidence digests or use fake-process startup timings as live
certification.
