# Current Task

## Task

**ID:** DOC-001
**Title:** Complete operator and compatibility documentation

## Objective

Close the M7 documentation gate with accurate fresh-checkout installation,
daily operator workflows, troubleshooting, and evidence-backed compatibility
claims for the implemented Yoctui product.

## Required work

1. Audit `README.md`, `docs/compatibility.md`, `docs/testing.md`, and all linked
   operator documents against the current CLI, UI specification, architecture,
   and verified live/fake boundaries.
2. Identify missing fresh-checkout, dependency installation, initialized Yocto
   environment, launch, workspace, build, editor, Devtool, maintenance,
   hardening, troubleshooting, and artifact instructions.
3. Because these are independent documentation outcomes, split genuine gaps
   into atomic child tasks with explicit verification before editing.
4. Keep commands directly copyable and distinguish Poky setup from standalone
   BitBake setup. Never claim a release or live workflow from fixture evidence.
5. Validate every internal link and shell snippet that can run without an
   external Yocto checkout.

## Definition of done

- Fresh-checkout installation and launch instructions are complete and
  reproducible.
- Daily workflows and troubleshooting cover all implemented destinations and
  safety boundaries.
- The compatibility matrix contains only evidence-backed claims and explicit
  limitations.
- All documentation verification and repository baseline checks pass.

## Verification

```bash
test -s docs/compatibility.md
test -s README.md
./scripts/verify-roadmap.sh
./scripts/check-checkout.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
```

## Documentation updates

- Split missing independent outcomes in `docs/task-registry.toml` before edits.
- Update `docs/implementation-status.md` with verified documentation evidence.
- Replace this file with the first atomic documentation child.

## Next task

To be selected by the documentation gap audit.
