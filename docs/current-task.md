# Current task

## Active task

**ID:** SEC-CAP-ADAPTER-001
**Title:** Inspect Security capabilities

## Objective

Construct fail-closed typed Security capability snapshots from explicit
initialized-workspace metadata and canonical host identities.

## Required work

1. Add a Security adapter module following existing explicit-snapshot
   capability inspectors.
2. Accept initialized build directory, release, exact available scopes,
   authoritative recipe tasks, image-SBOM configuration, report-root values,
   and PATH directories as typed input.
3. Preserve the exact reported `cve_check`, `create_recipe_sbom`,
   `create_spdx`, or image task rather than selecting from release text.
4. Canonicalize the build directory and report roots, refuse symlinks,
   non-directories, escapes, duplicates, relative paths, and excessive input.
5. Discover `cve-check-map-pkgs` only as a canonical regular executable in
   the supplied PATH snapshot and construct its exact bounded input arguments.
6. Return partial limitations for invalid optional roots/tools while failing
   closed for the primary build/scope identity.
7. Add fake-filesystem tests for current, legacy, missing, partial, unsafe,
   duplicate, and bounded capability input.

## Definition of done

- Capability identity is canonical and fail-closed.
- Current and legacy task names remain exact typed input.
- Optional roots and mapping support are explicit and bounded.
- No process-global environment is read or mutated.
- Focused capability and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake security_capability
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
