# Current Task

**ID:** DTB-DECOMPILE-DIALOG-RELEASE-001
**Title:** Release device-tree decompile destination workflow
**Status:** IN_PROGRESS

Publish the completed DTB/DTBO destination and successful-view workflow as
v0.1.239, build and install the optimized binary, and restart the initialized
Romulus daemon.

Verify with:

```bash
cargo fmt --all --check
cargo clippy -p yoctui-model --all-features -- -D warnings
cargo clippy -p yoctui-app --all-features -- -D warnings
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
```
