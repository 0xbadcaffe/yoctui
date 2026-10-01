# Current Task

**ID:** ROOTFS-SYSTEMD-SCROLL-001
**Title:** Keep the selected systemd service visible beyond the first viewport
**Status:** IN_PROGRESS

Dependency ROOTFS-TARGET-OWNERSHIP-001 is DONE in v0.1.258. Unrelated M67
performance evidence remains BLOCKED; global search marker correction follows.

Relevant files: rootfs_services.rs and focused UI/model/app tests, workspace
versions. Done when the bounded viewport follows existing service selection
through paging/wheel/reverse navigation and resize, clipped title cues and
empty/short/tiny cases pass with unchanged edit/explorer behavior. Update UI,
architecture, roadmap, registry and status; bump, commit, push and build release.

```bash
cargo test -p yoctui-model rootfs_systemd
cargo test -p yoctui-app rootfs_systemd
cargo test -p yoctui-ui rootfs_systemd
cargo test -p yoctui-ui ux_rootfs_system_tabs_show_offline_service_and_bus_file_evidence
cargo fmt --all --check
cargo clippy -p yoctui-ui --all-targets --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
cargo build --release -p yoctui --bin yoctui
```

Do not run the full suite or completion gate; the user deferred it. Preserve
daemon/build data and user captures. Continue to the search marker correction
immediately after the coherent commit.
