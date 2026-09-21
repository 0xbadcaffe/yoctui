# Current Task

**ID:** REDUCE-MODEL-WORKFLOWS-001
**Title:** Decompose image and build workflow domain models
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-APP-STATE-001 is DONE. Split rootfs.rs, wic.rs, sdk.rs,
image.rs and project_profile.rs into meaningful request, result, state and
validation modules. Preserve public APIs and typed workflow behavior while
targeting approximately 500 lines per source.

```bash
cargo test -p yoctui-model --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
