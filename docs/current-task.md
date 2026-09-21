# Current Task

**ID:** REDUCE-CLI-DAEMON-COMPAT-001
**Title:** Decompose daemon compatibility inspection
**Status:** NOT_STARTED

Dependency REDUCE-CLI-DAEMON-BITBAKE-001 is DONE. Split
daemon_compatibility.rs into named probe-planning, result-reduction and runtime
ownership modules targeting approximately 500 lines each. Move its inline
tests into the CLI test folder without changing authoritative fail-closed
evidence or process ownership behavior.

```bash
cargo test -p yoctui --all-features daemon_compatibility
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
