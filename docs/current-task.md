# Current task

## Active task

**ID:** QA-001
**Title:** Recipe, kernel, and layer QA workflows

## Objective

Implement the roadmap's recipe, kernel, and layer QA workflows as typed,
verified Yoctui operations.

## Required work

1. Inspect the existing QA-related behavior, authoritative UI specification,
   architecture, and tests before changing code.
2. Reconcile the broad roadmap item into atomic specification, model, adapter,
   rendering, CLI, and parent-gate tasks before implementation.
3. Cover kernel configuration, URI, patch, license, recipe QA, and
   `yocto-check-layer` workflows without free-form command authority.
4. Preserve exact target, task, provider, layer, report, and operation
   identities across every boundary.
5. Add unit, reducer, fake-process, CLI integration, and Ratatui TestBackend
   coverage appropriate to each atomic task.

## Definition of done

- QA workflows are split into coherent dependency-ordered tasks before broad
  implementation begins.
- Every implemented workflow uses typed previews, lifecycle state, and exact
  identities.
- Relevant focused and baseline checks pass without weakening tests.
- Fake-process evidence is not described as live Yocto compatibility.

## Verification

```bash
cargo test -p yoctui-app qa_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
