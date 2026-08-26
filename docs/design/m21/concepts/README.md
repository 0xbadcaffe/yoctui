# M21 workbench visual concepts

This directory contains the reviewed visual-direction pack for the M21
one-stop Yocto workbench. The images are real PNG raster mockups generated
before implementation. They illustrate hierarchy, density, focus, palette,
widget choice, and interaction states at the logical `160x50` terminal target.

These PNGs are not executable UI specifications and are not machine-regression
goldens. Generated text, glyph placement, and incidental values can be
imperfect. Implementations must take behavior and authority from
[`docs/ui-spec.md`](../../../ui-spec.md), typed model state, and deterministic
Ratatui `TestBackend` fixtures. No renderer may copy mockup-only data into
production state.

## Concept pack

| Scenario | Image | Design questions covered |
| --- | --- | --- |
| Idle dashboard | [`01-idle-dashboard.png`](01-idle-dashboard.png) | Shell density, project context, quick actions, resource meters, idle authority |
| Active build | [`02-active-build-tasks.png`](02-active-build-tasks.png) | Determinate progress, task table, following logs, history, telemetry, Inspector |
| Failed build | [`03-failed-build-errors.png`](03-failed-build-errors.png) | Failure hierarchy, correlated logs, paused search, scrolling, filters, recovery actions |
| Rootfs composition | [`04-rootfs-composition.png`](04-rootfs-composition.png) | Pie/table equivalence, package authority, checkboxes, tree drill-down, scrollbar |
| Editor and menu | [`05-editor-application-menu.png`](05-editor-application-menu.png) | F10 menu, focus trap, disabled reason, multiline editor, diagnostics, diff preview |
| Terminal sessions | [`06-terminal-sessions.png`](06-terminal-sessions.png) | Split PTYs, tabs, writer lease, read-only client, scrollback search, prefix help |

## Validation use

Each scenario has machine-readable metadata and review anchors in
[`manifest.toml`](manifest.toml). During implementation:

1. Build typed fixtures for the same scenario and logical terminal size.
2. Treat semantic anchors and the authoritative UI specification as required.
3. Compare hierarchy, density, focus, and color roles to the concept manually.
4. Serialize every Ratatui cell symbol and style for the actual regression
   golden.
5. Rasterize the deterministic cell buffer with a pinned font and renderer for
   PNG comparison and visual diff output.
6. Keep live PTY ANSI/text/metadata evidence separate from both concept and
   fixture images.

Exact pixel comparison against these generated concepts is prohibited. Exact
pixel comparison is appropriate only for deterministic implementation PNGs in
a pinned rendering environment. PNG is required; JPEG is not accepted for
goldens because lossy compression changes text pixels.

The same six scenario identities now render through Yoctui's production
`render_at` path at `160x50`. Every scene has a reviewed full cell/style golden
and a readable semantic capture under `crates/yoctui-ui/tests/golden`. The
manifest maps those real-renderer artifacts to semantic anchors and to explicit
open implementation gaps. A gap cannot remain after its owning registry task
is complete.

Verify the pack's declared files, dimensions, hashes, anchors, and lossless
format with:

```bash
./scripts/verify-m21-concept-pack.py
./scripts/verify-m21-concept-screens.sh
```

Intentional production-renderer changes use
`./scripts/update-m21-concept-screen-goldens.sh`; every resulting cell and text
diff must be reviewed. The update command never changes the generated PNGs.

The generation specifications are retained in [`prompts.md`](prompts.md).
