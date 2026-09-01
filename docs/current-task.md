# Current Task

## Task

**ID:** IMAGE-TARGET-AUTH-001
**Title:** Reject deployed files as image recipe targets
**Status:** DONE

## Objective

The Images workspace must submit only exact authoritative recipe identities to
BitBake, preserve requested target/outcome diagnostics, and prove that standard
non-minimal Poky image recipes remain buildable.

## Dependencies

- AUTH-ATTACH-001 — DONE

## Definition of done

- A selected deployed artifact builds only when its identity exactly matches
  an authoritative workspace recipe.
- Kernel, bootloader, metadata, and other non-recipe deploy entries cannot
  replace the current build target or open confirmation.
- Daemon job transitions retain the originally requested targets and status
  reports target plus terminal outcome.
- Standard non-minimal Poky image targets pass a live no-execute probe.
- Version 0.1.7 is installed and repository completion gates pass.

## Verification

```bash
cargo test -p yoctui-model image_artifact_build
cargo test -p yoctui-bitbake boot_artifact_identity
cargo test -p yoctui daemon_build_job_updates_preserve_the_requested_targets
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

The live Poky 6.0.2 / BitBake 2.18.0 no-execute probe accepted
`core-image-full-cmdline`, `core-image-sato`, and `core-image-weston`, planned
10,159 tasks, and completed with no errors. Version 0.1.7 focused and
repository-wide tests pass.
