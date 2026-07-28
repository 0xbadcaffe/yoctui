# Current task

## Active task

**ID:** SDK-TOOLS-001
**Title:** Adapt SDK publication and native tools

## Objective

Implement capability discovery, exact shell-free command reconstruction, and
bounded cancellable process execution for SDK publication and native-tool
workflows without sourcing scripts into the Yoctui process.

## Required work

1. Inspect the completed SDK request/preview/session model and existing
   Devtool, QEMU, Wic, package-data, and signature process adapters before
   writing code.
2. Add an SDK capability inspector in `yoctui-bitbake` that discovers
   `oe-publish-sdk`, `oe-find-native-sysroot`, and `oe-run-native` only as
   canonical regular executable non-symlink files beneath authoritative
   workspace roots. Preserve individual missing tools without guessing paths.
3. Add publication command construction that independently revalidates the
   exact selected regular installer, canonical absolute destination, tool
   executable, and model preview identity. Never use a shell, silently
   overwrite, or accept symlink/path escapes.
4. Add native command construction for `oe-find-native-sysroot` and
   `oe-run-native`. Validate the exact mode, bounded recipe/tool/argument
   identity, canonical active-build or extracted-SDK root, and one
   adapter-validated environment-setup file. Build a bounded child-only
   environment; never source a script or mutate the Yoctui process
   environment.
5. Add a single-operation asynchronous runner that emits typed started,
   bounded stream-tagged output, success, nonzero failure, cancellation,
   cancellation rejection/escalation, timeout, and process-loss events.
   Execution must use exact native argument vectors without a shell.
6. Add focused fake-filesystem/process tests named `sdk_tool` covering partial
   capability, unsafe/symlink/missing tools and paths, exact publication and
   native argv, tampered previews, extracted-root/environment validation,
   child-only environment isolation, output bounds, duplicate rejection,
   success, nonzero failure, timeout, graceful/forced cancellation, and loss.
   Do not claim live SDK compatibility.
7. Update `docs/architecture.md` only if the implemented ownership boundary
   differs from the existing Managed SDK boundary.
8. Run focused and baseline checks, then hand off to `SDK-RENDER-001`.

## Definition of done

- Capability and command construction accept only canonical adapter-validated
  tool, artifact, destination, and workspace identities.
- Publication and native tools execute directly with exact typed arguments and
  a child-only environment.
- Output and lifecycle are typed, bounded, cancellable, and independent from
  model/UI mutation.
- Focused fake adapters cover all terminal and validation outcomes.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake sdk_tool
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
