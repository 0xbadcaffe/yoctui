# Current Task

**ID:** FRESH-CLONE-001
**Title:** Allow setup to initialize a fresh build directory
**Status:** NOT_STARTED

Dependency SEARCH-CONTENT-001 is DONE. Files: bitbake build_environment.rs.
Done: validated fresh build destinations reach the selected setup script;
existing files and symlinks fail before execution, and missing source/script
paths remain errors. Add adapter regressions for fresh setup and failure paths.
Verify: `cargo test -p yoctui-bitbake build_environment` and AGENTS.md baseline.
Update specification, architecture, registry and status; commit then continue
DAEMON-STARTUP-002.
