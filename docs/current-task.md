# Current Task

**ID:** GLOBAL-SEARCH-MARKER-001
**Title:** Show one loading activity marker in global and workspace search
**Status:** IN_PROGRESS

Dependency ROOTFS-SYSTEMD-SCROLL-001 is DONE in v0.1.259. Ownership is DONE
in v0.1.258. M67 performance evidence remains externally BLOCKED.

Relevant files: palette_render.rs, state primitive and focused search tests,
workspace versions. Replace only the search loading state's default static
ellipsis with its existing client-local activity marker. Done when Unicode
frames animate exactly one marker, reduced-motion/ASCII fallbacks are safe,
other states/scopes remain unchanged, docs/status/version are updated, focused
checks and release build pass, and coherent commit/push/release handoff finish.

```bash
cargo test -p yoctui-ui global_search
cargo test -p yoctui-ui devtool_editor_search
cargo test -p yoctui-ui primitives
cargo fmt --all --check
cargo clippy -p yoctui-ui --all-targets --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
cargo build --release -p yoctui --bin yoctui
```

Full suite and completion gate remain deferred per user. Preserve running
daemon/build data and captures. Final release must include all three fixes;
reclaim only regenerable caches as needed and report final free space.
