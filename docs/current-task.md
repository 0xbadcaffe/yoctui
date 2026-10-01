# Current Task

**ID:** ROOTFS-TARGET-OWNERSHIP-001
**Title:** Display target RootFS owner/group and mode rather than host metadata
**Status:** IN_PROGRESS

Dependency ROOTFS-FILES-BROWSER-001 is DONE. User request supersedes M100 host
ownership presentation; unrelated M67 performance evidence remains BLOCKED.

Relevant files: bitbake RootFS browser/target metadata adapter and tests, model
RootfsFileMetadata/tests, shared RootFS renderer/tests, workspace versions.
Done when exact-root read-only Pseudo attributes match path/device/inode,
target passwd/group resolve root and non-root names, unavailable/corrupt/stale
data never falls back to host/root, navigation/preview/chart/Layers regressions
pass, real Romulus evidence is recorded, version is bumped and change committed,
pushed and source-bound release binary built. Update UI, architecture, roadmap,
registry and status in the coherent implementation commit.

```bash
cargo test -p yoctui-bitbake rootfs_browser
cargo test -p yoctui-model rootfs_browser
cargo test -p yoctui-model layer_tree
cargo test -p yoctui-app rootfs_browser
cargo test -p yoctui-ui rootfs_browser
cargo test -p yoctui-ui ux_rootfs_packages_pair_wide_pie_with_exact_table_and_accessible_fallbacks
cargo test -p yoctui --bin yoctui rootfs_browser
cargo fmt --all --check
cargo clippy -p yoctui --all-targets --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
cargo build --release -p yoctui --bin yoctui
```

Do not run the full suite or completion gate; the user deferred it. Live check
must read genuine Romulus Pseudo/account data without restarting the daemon,
mutating a build or starting BitBake.
