# Current Task

## Task

**ID:** UI-LIVE-POKY-001
**Title:** Validate the recovered workbench against live Poky
**Status:** IN_PROGRESS

## Objective

Add and pass a repeatable live-Poky gate that verifies typed workspace, layer,
and recipe discovery plus the colored wide workbench's essential visual anchors.

## Dependencies

- `UI-LIVE-DISCOVERY-001` — DONE

## Relevant files

- `scripts/test-live-workbench.sh`
- `crates/yoctui-cli/src/main.rs`
- `artifacts/release-quality/`
- `docs/task-registry.toml`
- `docs/implementation-status.md`

## Definition of done

- Live bridge inspection reports Poky MACHINE, DISTRO, and release.
- Live layer discovery includes the configured Poky layers.
- Live recipe discovery includes `core-image-minimal` and `busybox`.
- A private-XDG colored wide PTY snapshot contains workbench, metadata, and
  focus-route anchors without a centered daemon-unavailable notice.
- Workspace tests, Clippy, bridge tests, and roadmap checks pass.

## Verification

```bash
./scripts/test-live-workbench.sh "$HOME/src/poky/build"
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```
