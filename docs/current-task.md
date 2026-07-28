# Current task

## Active task

**ID:** WIC-ADAPTER-CAP-001
**Title:** Inspect Wic capability and construct creation commands

## Objective

Add canonical Wic and kickstart discovery, bounded typed partition preview
parsing, and independently revalidated shell-free cooked-mode command specs.

## Required work

1. Record that the active BitBake-only source has no Wic executable/canned
   kickstarts; use fake fixtures and do not claim live compatibility.
2. Add `yoctui-bitbake::wic` capability inspection for an explicit executable
   or active PATH plus bounded deterministic `wic list images` parsing and
   configured canonical `.wks`/`.wks.in` files.
3. Read kickstart sources without following symlinks, bound bytes/lines, and
   parse only typed `part`/`partition` fields needed by the preview. Preserve
   unsupported/dynamic syntax as explicit limitations.
4. Independently rebuild the exact cooked-mode native argument vector from a
   model preview and reject path, identity, option, or argument tampering.
5. Add fake-process/filesystem tests named `wic_adapter_capability` for missing,
   malformed, oversized, symlink, configured, canned, partial, exact-command,
   and tampered-preview paths.
6. Add mechanical app normalization for typed capability results, run focused
   and baseline checks, then mark the child done and hand off to
   `WIC-ADAPTER-RUNNER-001`.

## Definition of done

- Capability and kickstart previews are canonical, bounded, and typed.
- Creation commands are independently validated and shell-free.
- Fake coverage is not presented as live compatibility.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake wic_adapter_capability
cargo test -p yoctui-app wic_adapter_capability
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
