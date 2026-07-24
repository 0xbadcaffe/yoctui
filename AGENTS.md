# Yoctui Agent Operating Contract

This file is the execution contract for autonomous coding agents working in this repository.

## Mandatory startup order

Before changing code, read these files in order:

1. `AGENTS.md`
2. `docs/current-task.md`
3. `docs/ui-spec.md`
4. `docs/architecture.md`
5. `docs/product-roadmap.md`
6. `docs/implementation-status.md`
7. `docs/task-registry.toml`

Do not begin implementation until the active task and its verification commands are understood.

## Source of truth

- `docs/ui-spec.md` is authoritative for UI behavior.
- `docs/architecture.md` is authoritative for component boundaries.
- `docs/task-registry.toml` is authoritative for task state and dependencies.
- `docs/current-task.md` contains exactly one active task.
- `docs/implementation-status.md` is the human-readable progress report.
- BitBake and Yocto metadata remain authoritative for build state.

When documents disagree, stop implementation and reconcile them in one governance commit before continuing.

## Autonomous execution loop

Treat this as executable pseudocode:

```text
while ./scripts/verify-completion.sh != PASS
    read docs/current-task.md
    verify its dependencies are DONE
    implement only that task
    add or update tests
    run the task verification commands
    update docs/task-registry.toml
    update docs/implementation-status.md
    replace docs/current-task.md with the next eligible task
    commit the coherent change
    continue immediately
end
```

After every successful commit:

1. Read `docs/current-task.md`.
2. Select the next highest-priority incomplete task whose dependencies are complete.
3. Begin it immediately.
4. Do not return a progress summary merely because a commit succeeded.
5. Do not ask for confirmation unless user intent is genuinely ambiguous or an irreversible external action is required.
6. Stop only when the completion gate passes or progress is blocked by a documented external dependency.

## Task discipline

Each implementation task must be small enough for one coherent commit.

A valid task has:

- a stable ID
- one concrete outcome
- explicit dependencies
- relevant files
- a definition of done
- verification commands
- required documentation updates

Do not combine unrelated tasks in one commit.

If a task is too large, split it in `docs/task-registry.toml`, update `docs/implementation-status.md`, set one child task as current, and commit the split before implementation.

## Definition of done

A task is `DONE` only when:

- required behavior exists
- tests cover normal and relevant failure paths
- verification commands pass
- intentional UI changes are reflected in `docs/ui-spec.md`
- architecture changes are reflected in `docs/architecture.md`
- task state is updated in `docs/task-registry.toml`
- the human-readable status is updated
- no temporary debug code remains
- the change is committed

Code existing without verification is `IN_PROGRESS`, not `DONE`.

## UI rules

- Do not invent layouts, dialogs, focus behavior, shortcuts, animations, or themes outside `docs/ui-spec.md`.
- Update `docs/ui-spec.md` in the same commit as every intentional UI behavior change.
- UI widgets consume typed model state and typed backend events.
- Ratatui widgets must not parse raw BitBake or process text.
- Dialogs trap focus.
- Destructive operations require preview and explicit confirmation.
- Narrow terminals must degrade safely and must never panic.

## Architecture rules

Dependency direction:

```text
yoctui-model
    ↑
yoctui-protocol
    ↑
yoctui-bitbake
    ↑
yoctui-app
    ↑
yoctui-ui
    ↑
yoctui CLI
```

The exact workspace may use additional support crates, but:

- model contains pure domain state and reducer logic
- protocol owns stable wire types
- bitbake owns process and bridge adapters
- app maps input and effects
- UI renders state and emits typed actions
- CLI owns startup, configuration loading, and terminal lifecycle

Backend code must not mutate UI state directly.

## Testing rules

For every new behavior:

- add unit tests for pure logic
- add reducer tests for state transitions
- add `TestBackend` tests for UI behavior
- add fake-process or fake-bridge tests for integrations
- add live-Yocto smoke coverage when required by the task

Required baseline checks:

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

Use task-specific commands from `docs/current-task.md` in addition to the baseline.

## Commit policy

Commit author:

```text
Roy Cohen <misteroy@gmail.com>
```

Use small, descriptive commits.

Examples:

```text
Add persistent background job model
Implement runqemu launch dialog
Test narrow terminal shell
Document bridge cancellation semantics
```

Do not use vague messages such as `updates`, `fix`, or `continue`.

## Handling blockers

When an external dependency prevents completion:

1. Change the task status to `BLOCKED`.
2. Record the exact dependency and reproduction details.
3. Add a follow-up verification command or manual validation requirement.
4. Select the next eligible task that does not depend on the blocker.
5. Continue.

Do not mark a blocked task `DONE`.

## User instructions

New user instructions override the current queue.

When the user requests a product or UI change:

1. Pause unrelated work.
2. Update the authoritative specification.
3. Create or reprioritize atomic tasks.
4. Implement and test the request.
5. Update roadmap state.
6. Commit.
7. Resume the queue.

## Prohibited shortcuts

Do not:

- mark broad parent tasks complete because one sub-feature works
- replace typed workflows with an unstructured shell textbox as the main UX
- weaken tests or completion checks to make the gate pass
- silently ignore missing optional tools
- claim live BitBake compatibility using only mocked tests
- stop after a commit while eligible tasks remain
