# Current Task

**ID:** M67-LIVE-EVIDENCE-001
**Title:** Refresh genuine current-source real-Poky performance evidence
**Status:** BLOCKED

M73 source decomposition is complete at v0.1.195. The maintained inventory has
2,670 Rust, Python and shell files, every source is at most 500 lines, Rust test
bodies live in test folders, and the layout gate enforces those boundaries.
No eligible implementation task remains.

The remaining required task depends on a genuine current-source and
binary-bound Yocto 6.0.2 `linux-yocto:do_compile` performance capture. The
retained evidence is bound to source base
`d2214e82974a5be708a7cc40f1532254d7c7de63` and now has 143 source digest
mismatches. Historical evidence and the user's running build/captures must
remain intact. Follow the capture procedure in `docs/performance.md`, then run:

```bash
./scripts/verify-performance.sh --real-poky-evidence
./scripts/verify-completion.sh
```

Do not rewrite retained digests or claim live compatibility from mocked tests.
