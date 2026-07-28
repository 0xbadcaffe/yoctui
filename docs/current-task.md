# Current task

## Active task

**ID:** QEMU-ADAPTER-001
**Title:** Detect and execute runqemu safely

## Objective

Add authoritative runqemu capability inspection, exact shell-free command
translation, bounded asynchronous process events, and process-group
cancellation without integrating terminal UI or claiming live compatibility.

## Required work

1. Inspect the existing Devtool process runner, image artifact adapter,
   executable-discovery helpers, cancellation tests, and QEMU model boundary
   before adding overlapping process code.
2. Inspect runqemu only from explicit configured candidates or the active
   process environment; distinguish available, missing tool, missing compatible
   deployed image, and failed inspection.
3. Correlate compatible images by exact typed artifact identity and supported
   root-filesystem/Wic kind. Reject relative, missing, symlinked, escaped, or
   stale paths instead of guessing.
4. Translate a validated `QemuLaunchPreview` into an executable plus
   `Vec<OsString>` argument vector. Never construct or invoke a shell command.
5. Start at most one child in the active build directory, assign a process
   group on Unix, and emit typed starting/started, bounded stream-tagged output,
   completed, failed, cancelled, cancellation-rejected, and lost events.
6. Bound individual output lines and event-channel retention, preserve invalid
   UTF-8 lossily, and mark truncation explicitly.
7. Cancel the child process group with graceful termination followed by bounded
   forced escalation. Reject duplicate starts and duplicate cancellation.
8. Keep process ownership in `yoctui-bitbake`; do not mutate model/UI state,
   parse output in widgets, or add CLI/TUI routing in this child.
9. Add fake-process tests named `qemu_adapter` for capability states, exact
   arguments, unsafe paths/options, output bounds, nonzero exit, duplicate
   start, cancellation, escalation, and process loss.
10. Update `docs/architecture.md`, then mark the child done and hand off to
    `QEMU-UI-001`.

## Definition of done

- Capability inspection and process execution are typed, bounded, shell-free,
  cancellable, and covered by fake integration tests.
- No live runqemu compatibility is claimed from fake tests.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake qemu_adapter
cargo test -p yoctui-app qemu_adapter
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
