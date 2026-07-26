# Current task

## Active task

**ID:** DEVTOOL-META-001
**Title:** Add authoritative Devtool workspace and Git status

## Objective

Add typed, authoritative Devtool capability, workspace membership/path, and
Git state for Recipes and Devtool views.

## Required work

1. Inventory current recipe workspace-status fields, Devtool source path
   assumptions, process ownership, Git scanning, Recipes Inspector rendering,
   and tests.
2. Define typed capability and status records for Devtool executable
   availability, recipe workspace membership/path, and Git repository state:
   branch/head, clean/dirty, modified/untracked/conflicted counts, and explicit
   unavailable/error reasons.
3. Implement external Devtool/Git inspection in the backend adapter boundary.
   Parse process output there and emit typed model data; widgets and reducers
   must not parse raw text.
4. Correlate status by absolute recipe identity and ignore stale responses.
   Missing workspace directories, non-Git sources, missing executables,
   malformed output, and non-zero exits must remain distinct.
5. Render status and exact action availability/disabled reasons in Recipes and
   the Devtool view across responsive modes.
6. Add fake-process, reducer, app, and TestBackend tests named
   `devtool_metadata`, including partial/malformed/error states.
7. Validate status against the available live Yocto Devtool workspace when
   external build-directory access is available; otherwise document the exact
   external blocker without marking the task done.

## Definition of done

- Devtool and Git status cross the backend boundary as typed data.
- Absolute recipe identity drives status and availability.
- Missing tool/workspace/Git, dirty/untracked/conflicted, malformed, and failed
  states are distinct.
- Recipes and Devtool views render responsive status and disabled reasons.
- Live status evidence is recorded or the task remains explicitly blocked.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-bitbake devtool_metadata
cargo test -p yoctui-model devtool_metadata
cargo test -p yoctui-app devtool_metadata
cargo test -p yoctui-ui devtool_metadata
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEVTOOL-JOBS-001 — Run Devtool operations as persistent background jobs`
