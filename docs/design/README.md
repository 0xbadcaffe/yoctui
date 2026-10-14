# Design and UI regression fixtures

Start with [Architecture](../architecture.md) for crate boundaries, state
ownership, IPC, and lifecycle. [Interface behavior](../interface-behavior.md) defines
focus, responsive layouts, and accessibility. [Widgets and dependencies](../widgets-and-dependencies.md)
explains the interaction and widget choices; [Testing](../testing.md) covers
the verification commands.

## Rendering and state ownership

The model owns selection, scrolling, focus, job state, and authoritative values.
Renderers project typed state into Ratatui buffers. Narrow layouts preserve
exact text and keyboard access when charts or optional panes do not fit.
The inspector is off by default; `Alt+i` toggles it for the session and
Preferences can change its startup default.

Compact Dashboard and Tasks layouts retain CPU, RAM, and build-filesystem
readings in a four-row fallback. Missing readings stay unavailable and zero
remains a valid measurement. Rendering and mouse selection share panel heights.
`compact_resource_meters_remain_visible_across_workspace_sizes` covers this
behavior across narrow layouts, tiny bounds, ASCII/no-color, and invalid values.

The daemon owns PTYs, the terminal emulator, scrollback, resize, and writer
leases. Validated sparse protocol cells expand into one client replica.
`tui-term` renders that replica through transient `Screen`/`Cell` adapters;
its optional parser and PTY controller are disabled. The UI never parses raw
terminal bytes or retains another emulator. Model, protocol, application,
TestBackend, and real-PTY tests verify cell styles, Unicode, cursor placement,
resize, focus, and session ownership.

Deploy artifacts have no raster MIME authority, so the Images workspace uses
exact metadata and rootfs composition rather than terminal image probing.
A future preview requires typed raster identity, bounded cancellable decoding,
and deterministic text fallbacks. See the dependency decisions in
[Widgets and dependencies](../widgets-and-dependencies.md#third-party-dependency-and-license-gate).

## UI regression scenes

[ui-scenes.toml](ui-scenes.toml) connects six typed production-renderer fixtures
to their semantic anchors, required features, cell goldens, and raster hashes.
The fixtures render at `160x50`; content scenes enable the optional inspector,
while terminal scenes reserve the workspace for the PTY panes.

| Scene | Reviewed production rendering |
| --- | --- |
| Dashboard | [Screenshot](../media/screenshots/07-idle-dashboard.png) |
| Active tasks | [Screenshot](../media/screenshots/01-active-build-tasks.png) |
| Failed build | [Screenshot](../media/screenshots/08-failed-build-errors.png) |
| Rootfs composition | [Screenshot](../media/screenshots/06-rootfs-composition.png) |
| Editor and application menu | [Screenshot](../media/screenshots/09-editor-application-menu.png) |
| Terminal sessions | [Screenshot](../media/screenshots/10-terminal-sessions.png) |

Exact cell symbols and styles under `crates/yoctui-ui/tests/golden` are the
regression authority. The screenshots are deterministic renderings of those
cells. [screenshots/manifest.toml](screenshots/manifest.toml) pins the renderer,
fonts, geometry, source hashes, and PNG hashes. The PNG files are shared with
the README gallery rather than duplicated here. Fixture images do not certify
live builds or performance.

```bash
./scripts/verify-m21-concept-screens.sh
./scripts/render-m22-concept-screenshots.sh --check
python3 scripts/test-m22-concept-raster.py
```

The script names remain stable for existing development workflows. After an
intentional UI change, run `./scripts/update-m21-concept-screen-goldens.sh`,
then `./scripts/render-m22-concept-screenshots.sh --update`. Review every cell,
semantic capture, and raster diff. Re-run the checks and the README raster
checks before committing; [Testing](../testing.md#ordinary-verification) lists
the wider test suite.

## Executable design contracts

[ui-scenes.toml](ui-scenes.toml) defines required features and semantic anchors.
Checks validate the production cell dimensions/styles, feature anchors, and
source/raster hashes. They do not rely on historical task IDs or progress stamps.
Fixture checks supplement executable tests; live acceptance requires fresh,
attributed interactions and assertions.
