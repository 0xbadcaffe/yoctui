# Current Task

## Task

**ID:** DOC-OPERATOR-001
**Title:** Document daily operator workflows and troubleshooting

## Objective

Give an operator one linked, task-oriented guide for using Yoctui as the daily
Yocto workspace while preserving the distinction between authoritative live
evidence and fixture-only behavior.

## Required work

1. Inspect the implemented screens, shortcuts, footer behavior, dialogs, and
   README before describing them; do not promise unfinished workflows.
2. Add `docs/operator-guide.md` covering persistent navigation and the normal
   build, task, log, error, image, layer, recipe, configuration, and editor
   workflows.
3. Cover Devtool, dependencies, signatures, package data, SDK, QEMU, Wic,
   Testing, Security, QA, and Maintenance with their capability/readiness,
   preview/confirmation, cancellation, evidence, and live-validation limits.
4. Document inherited-shell and external-editor transitions, Settings and
   session/config precedence, background-job navigation, cancellation, and
   terminal outcome inspection.
5. Add actionable troubleshooting for missing workspace metadata, unsupported
   backend capability, missing tools/artifacts, failed or lost jobs, terminal
   restoration, and safe diagnostics; link rather than duplicate installation,
   compatibility, testing, and profiling details.
6. Link the guide prominently from `README.md`.

## Definition of done

- `docs/operator-guide.md` is task-oriented, complete for implemented daily
  workflows, and explicit about confirmation and authority boundaries.
- Missing capability, evidence, cancellation, and failure paths are actionable.
- README links the guide and no documented shortcut contradicts the UI spec.
- Task and baseline verification pass.

## Verification

```bash
test -s docs/operator-guide.md
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Add `docs/operator-guide.md` and link it from `README.md`.
- Mark `DOC-OPERATOR-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `DOC-COMPAT-001`.

## Next task

`DOC-COMPAT-001`
