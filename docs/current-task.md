# Current task

## Active task

**ID:** QA-TASK-ADAPTER-001
**Title:** Adapt recipe and kernel QA capabilities

## Objective

Construct fail-closed typed Recipe & Kernel QA capability snapshots from
authoritative initialized Yocto metadata without guessing task names, provider
paths, kernel status, report roots, or release behavior.

## Required work

1. Inspect the QA model constructors, recipe metadata adapter, workspace/layer
   snapshots, capability adapters, fake filesystem/process patterns, and the
   authoritative QA architecture before writing code.
2. Add a focused BitBake adapter module that accepts an exact build identity,
   selected recipe/provider, eligible recipe scopes, authoritative task
   inventories, explicit kernel classification, and explicit report-root
   candidates.
3. Emit all required kernel configuration, URI, patch, license, and
   recipe/package catalog families for each exact scope, using only an exact
   task explicitly reported for that family and scope.
4. Keep unsupported checks as disabled typed catalog rows with stable reasons;
   never derive support from recipe names, provider filenames, release strings,
   similar task spelling, inherited classes, or report filenames.
5. Canonicalize and revalidate build/provider/report-root identities, reject
   symlinks and escapes, bound all inventories, and preserve usable scopes as
   partial capability with exact limitations when optional inputs are unsafe.
6. Add adapter tests for current/legacy or alternate reported tasks as data,
   missing tasks, non-kernel scope, invalid/stale provider and roots,
   duplicates/bounds, partial capability, exact BuildRequest reconstruction,
   and no guessed fallback.
7. Add mechanical app response mapping only if a new typed adapter response is
   required; do not implement report parsing, layer execution, CLI polling, or
   final rendering in this task.

## Definition of done

- Capability snapshots contain only exact validated build, scope, provider,
  task, and report-root identities.
- Every required family remains visible and unavailable rows explain why.
- Kernel-only capability is driven by explicit authoritative classification.
- Focused adapter/app and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake qa_task
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
