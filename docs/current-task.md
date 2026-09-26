# Current Task

**ID:** HEADER-STATUS-001
**Title:** Move local time and daemon status into persistent Header chrome
**Status:** IN_PROGRESS

Replace the UTC-seconds Footer clock with host-local system time in HH:MM form
in the Header. Move the current daemon message beneath the version in bold,
high-visibility semantic color with activity while waiting; when no transient
message exists, show daemon health there. Leave the Footer to shortcuts and
preserve responsive narrow-terminal bounds.

Verify with:

```bash
cargo test -p yoctui-ui header_status
cargo test -p yoctui-app concept_chrome
cargo fmt --all --check
```
