# Current Task

## Task

**ID:** DEVWORK-VALIDATION-001
**Title:** Validate the complete selected-recipe development loop
**Status:** DONE

## Objective

Run the repository-wide quality gates, verify the selected-recipe development
loop on the initialized Poky environment, install the release binary, and
publish the completed milestone.

M23 and its parent gate `DEVWORK-001` are complete. The next task is selected
from new user scope or a newly registered milestone; no required registry task
remains open.

## Dependencies

- DEVWORK-EDITOR-001 — DONE
- DEVWORK-TERMINAL-001 — DONE

## Definition of done

- Workspace tests and all-feature Clippy pass without warnings.
- Roadmap, UI contract, completion, and repository cleanliness gates pass.
- Live initialized-environment status remains truthful for Devtool and PTYs.
- A second concept-to-production pass has current cell/style rasters, aligned
  Navigator/workspace identity, and no stale semantic anchors.
- A release binary containing M23 is installed and smoke-tested.

## Verification

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
./scripts/verify-m22-concept-parity.sh
./scripts/verify-completion.sh
```
