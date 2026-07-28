# Current task

## Active task

**ID:** WIC-ADAPTER-001
**Title:** Discover kickstarts and execute Wic creation safely

## Objective

Add canonical Wic/kickstart capability discovery, bounded kickstart partition
preview parsing, independent cooked-mode command revalidation, cancellable
execution, and exact generated-output scanning.

## Required work

1. Inspect the QEMU/image adapters and current Wic scripts in the active Yocto
   source before writing code.
2. Add `yoctui-bitbake::wic` capability inspection for an explicit executable
   or active PATH plus bounded deterministic `wic list images` parsing and
   configured canonical `.wks`/`.wks.in` files.
3. Read kickstart sources without following symlinks, bound bytes/lines, and
   parse only typed `part`/`partition` fields needed by the preview. Preserve
   unsupported/dynamic syntax as explicit limitations.
4. Independently rebuild the exact cooked-mode native argument vector from a
   model preview and reject path, identity, option, or argument tampering.
5. Add a one-child process-group runner with bounded stdout/stderr, timeout,
   graceful/forced cancellation, duplicate rejection, and typed events.
6. Snapshot the output directory before launch and after successful completion;
   return only new canonical regular non-symlink files beneath the exact root,
   with typed kind, size, and modification time. Report empty/partial results
   honestly.
7. Add fake-process/filesystem tests named `wic_adapter` for discovery, parsing,
   exact arguments, output scanning, malformed/oversized/symlink inputs, every
   terminal outcome, and cancellation.
8. Add mechanical app normalization for adapter events, run focused and
   baseline checks, then mark the child done and hand off to
   `WIC-UI-MODEL-001`. Do not claim live Wic compatibility from fake tests.

## Definition of done

- Capability and kickstart previews are canonical, bounded, and typed.
- Creation commands are independently validated and shell-free.
- Runner output and lifecycle are bounded, cancellable, and typed.
- Only exact new files under the requested output root become typed outputs.
- Fake coverage is not presented as live compatibility.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake wic_adapter
cargo test -p yoctui-app wic_adapter
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
