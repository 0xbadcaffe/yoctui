# Current Task

**ID:** SEARCH-CONTENT-001
**Title:** Restrict global search to build text content and start empty
**Status:** NOT_STARTED

Dependencies: none. Scope: model search projection, CLI scanner and UI palette.
Done: `/` opens empty, never lists commands, and matches only file contents in
build directories including generated rootfs and text image artifacts. Blank
queries do no filesystem work; invalid regex and stale results remain bounded.
Verify: `cargo test -p yoctui global_search`, `cargo test -p yoctui-model global_`,
`cargo test -p yoctui-ui global_search`, and the AGENTS.md baseline.
Update UI specification, architecture, registry and implementation status.
Next: FRESH-CLONE-001, then DAEMON-STARTUP-002.
