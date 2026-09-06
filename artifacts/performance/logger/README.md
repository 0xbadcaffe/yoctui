# tui-logger integration checkpoint

Version: 0.1.60. Compiler: rustc 1.97.0 (2d8144b78).

The existing pre-integration release matrix at commit `9dbbbecc` recorded
853,489 ns/frame for log-heavy output (500 frames, 160x48). This is a historical
comparison reference, not a newly measured 0.1.59 baseline.

The new adapter completed this development-profile workload:

```bash
YOCTUI_PROFILE_FRAMES=300 YOCTUI_PROFILE_SCENARIO=log-heavy \
  cargo bench --profile dev -p yoctui --bench workbench_profile --all-features
```

Observed: 300 frames, 6,674 ms, 22,248,286 ns/frame,
checksum `de610b277d5bd93d`. Debug timing must not be compared directly with
release timing or presented as satisfying the 10-ms release rendering gate
or the one-logical-CPU overhead target. The final M52 parent gate runs the
release matrix and independent CPU/responsiveness gates.

Measured development binary SHA-256:
`78dedda4397a7be23b271f89db2f10ec03d4d0af1845a5d6f07274e56f44e7da`.

Adapter source SHA-256:
`4e97d15b33d977d841390a2197bbdf10928b0e43d7bf29c726cc64db65d328e8`.

The complete UI suite passed before blessing version-only header updates.
Production cell/text fixtures changed only from 0.1.59 to 0.1.60; wrapping,
columns, selection and log semantics are unchanged. New tests independently
cover bounded projection, long Unicode payloads, word wrapping, concurrent
pane isolation, more than one hot-buffer batch, and no-color rendering.
