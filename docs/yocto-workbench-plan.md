# Yocto workbench roadmap

Yoctui is the one-stop terminal workspace after `oe-init-build-env`: users should be able to build images, follow package progress, inspect and edit supported workspace files, and invoke BitBake/Devtool operations without leaving the TUI. BitBake and Devtool remain authoritative for metadata and workspace changes. Every future write operation must be an explicit user action with a preview or confirmation where it changes a workspace.

## One-stop workspace principles

- Build cockpit: start image tasks for the effective machine, track all observed package tasks, inspect host capacity, and fall back to the inherited Yocto shell when a direct command is needed.
- Orientation: keep the active Yocto release/workspace visible and expose the relevant shortcuts in the bottom bar on every screen.
- Workspace changes: browse layers and configuration, open supported sources, and use Devtool-backed editing rather than guessing metadata ownership.
- Safe configuration: show BitBake's effective values and provenance first; only then add narrowly scoped edits with a diff preview, confirmation, and metadata refresh.
- Trust boundary: Yoctui never reimplements BitBake dependency resolution or silently writes generated build state.

## Build cockpit

- [DONE] Display active BitBake package tasks as colored progress gauges with percentages on the dashboard.
- [DONE] Provide an inherited Yocto shell (`!`) plus a machine-aware image build-options menu (`B`) for build, clean, menuconfig, and target selection.
- [DONE] Refresh host CPU utilization and available space for the build-directory filesystem while a build is active.
- [DONE] Retain a bounded completed-task history so the dashboard can display active, successful, and failed package tasks for the current build; scroll it with `Up`/`Down`.
- [NOT_STARTED] Add build queue, task failure drill-down, and build-history views backed by BitBake events.

## Recipe workspace editor

- [IN_PROGRESS] Devtool-backed in-TUI source editor: `d` prepares a Devtool workspace, displays a two-pane file editor, supports editing and `Ctrl+S`, and returns with `Esc`.
- [DONE] Confirmation-protected `devtool reset` from the Recipes screen.
- [NOT_STARTED] Resolve and display original `.bb`/`.bbappend` paths from BitBake so the left tree can include the providing metadata layer as well as the Devtool source workspace.
- [IN_PROGRESS] Add Devtool `finish`, `update-recipe`, and deploy workflows, each with a preview and confirmation. `u` runs confirmation-protected `devtool update-recipe`; `F` selects a destination layer and runs confirmation-protected `devtool finish`; both refresh workspace metadata. Deploy still needs target selection.

## Metadata and graph views

- [DONE] Clearly identify every backend-supplied layer as active in the current build configuration, with color highlighting where enabled.
- [NOT_STARTED] Add an on-demand dependency graph view backed by BitBake-generated graph data (`bitbake -g` or a supported server query), with recipe/task graph navigation and no independent dependency resolution.
- [NOT_STARTED] Add read-only layer relationship views: priorities, compatibility, overlays, appends, and declared dependencies supplied by BitBake.
- [NOT_STARTED] Add configuration provenance chains that distinguish original, append, override, and effective values when the active BitBake server supplies them.

## Explicit configuration controls

- [DONE] Add a read-only BBMASK view (`x`), populated by BitBake's effective configuration and provenance when supplied by the backend.
- [DONE] Add an opt-in BBMASK editing dialog that previews the exact `conf/local.conf` assignment, writes only after confirmation, and refreshes BitBake workspace metadata afterward.
- [NOT_STARTED] Add equivalent preview/confirmation workflows for supported Devtool and layer operations; Yoctui must never silently modify configuration.

## Verification

- [NOT_STARTED] Validate these workflows against a supported, real Yocto/BitBake environment and record the compatibility matrix.
