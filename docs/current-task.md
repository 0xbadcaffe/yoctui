# Current Task

## Task

**ID:** DEVWORK-VALIDATION-001
**Title:** Validate the complete selected-recipe development loop
**Status:** IN_PROGRESS

## Objective

Run the repository-wide quality gates, verify the selected-recipe development
loop on the initialized Poky environment, install the release binary, and
publish the completed milestone.

## Dependencies

- DEVWORK-EDITOR-001 — DONE
- DEVWORK-TERMINAL-001 — DONE

## Definition of done

- Workspace tests and all-feature Clippy pass without warnings.
- Roadmap, UI contract, completion, and repository cleanliness gates pass.
- Live initialized-environment status remains truthful for Devtool and PTYs.
- A release binary containing M23 is installed and smoke-tested.

## Verification

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```
