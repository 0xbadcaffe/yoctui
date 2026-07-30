# Current task

## Active task

**ID:** SEC-001
**Title:** CVE and SPDX workflows

## Objective

Implement typed CVE mapping/check and SPDX report generation/viewing
workflows.

## Required work

1. Inspect existing recipe CVE/SPDX actions and security-related model,
   adapter, app, UI, and CLI behavior before adding code.
2. Reconcile the requested security workflows with the authoritative UI and
   architecture contracts.
3. Split this parent task into atomic specification, model, adapter, renderer,
   CLI, and integration tasks if it cannot fit one coherent commit.
4. Implement only the selected atomic task and add the applicable tests.

## Definition of done

- CVE mapping/checks and SPDX report generation/viewing are typed workflows.
- Process and metadata output is parsed outside widgets.
- Operations remain cancellable, identity-correlated, and responsive.
- Relevant focused and baseline verification passes.

## Verification

```bash
cargo test -p yoctui-app security_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
