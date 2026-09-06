# Yocto log presentation

Yoctui uses [tui-logger](https://github.com/gin66/tui-logger) 0.18.3 for
BitBake Logs, Dashboard/Tasks log panes and recent tails, correlated failure
logs, captured Raw stdout/stderr, and managed-tool output in the relevant
workspaces and inspectors. The dependency is MIT-licensed; its packaged
license and resolved dependencies are included in the generated
[third-party notices](compliance/THIRD_PARTY_NOTICES.md) and
[shipped SBOM](compliance/yoctui.cdx.json).

Use Logs for follow/pause, severity/build/recipe/task/source/time filtering,
search, bookmarks, horizontal scrolling, wrapping, and export. These controls
and retained records still belong to Yoctui's typed model. Rendering cannot
add duplicate records or change build/task outcomes. A temporary projection
is limited to the visible rows and 256 KiB; unusual oversized projections
show `[viewport truncated]` without deleting the original retained record.

The widget does not install a global logger, start a log polling thread, or
write a second log file. Yoctui's own tracing diagnostics remain in the
separate Yoctui log tab. This integration covers logs acquired by Yoctui; it
does not automatically crawl every log file on the host or inside an image.

SSH and runqemu are interactive terminals rendered by
[tui-term](https://github.com/a-kenji/tui-term), not log viewers. See
[embedded shells](embedded-shell.md) for Images `T` and advanced `Q` launch
options. Image-installed udev rules are under Images → `6` udev rules; see
[rootfs composition](rootfs-composition.md).

## Regression checks

```bash
cargo test -p yoctui-ui --all-features yocto_logger
cargo test --workspace --all-features log
./scripts/verify-widget-dependencies.sh
./scripts/verify-third-party-notices.sh
cargo deny check
```

Tests cover parallel/repeated pane isolation, more than 64 visible rows,
no-color rendering, word wrapping against Ratatui's previous paragraph
layout, Unicode/search styles, and projection bounds. Existing production
goldens continue to validate log labels, correlation, selection and layout.
The upstream crate does not declare a minimum Rust version. This admission
uses Rust 1.97.0 as its tested supported compiler floor; older compilers are
not certified by these tests.

Release rendering and CPU/latency validation remain independent gates in
[the performance contract](performance.md). Fixture output is not evidence
of a real SSH login, QEMU guest boot, or real BitBake build.
