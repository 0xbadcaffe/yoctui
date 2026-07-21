# Yocto workbench roadmap

Yoctui remains a BitBake frontend: BitBake and Devtool are the authoritative interfaces for metadata and workspace changes. Every future write operation must be an explicit user action with a preview or confirmation where it changes a workspace.

## Recipe workspace editor

- [IN_PROGRESS] Devtool-backed in-TUI source editor: `d` prepares a Devtool workspace, displays a two-pane file editor, supports editing and `Ctrl+S`, and returns with `Esc`.
- [DONE] Confirmation-protected `devtool reset` from the Recipes screen.
- [NOT_STARTED] Resolve and display original `.bb`/`.bbappend` paths from BitBake so the left tree can include the providing metadata layer as well as the Devtool source workspace.
- [NOT_STARTED] Add Devtool `finish`, `update-recipe`, and deploy workflows, each with a preview and confirmation.

## Metadata and graph views

- [NOT_STARTED] Add an on-demand dependency graph view backed by BitBake-generated graph data (`bitbake -g` or a supported server query), with recipe/task graph navigation and no independent dependency resolution.
- [NOT_STARTED] Add read-only layer relationship views: priorities, compatibility, overlays, appends, and declared dependencies supplied by BitBake.
- [NOT_STARTED] Add configuration provenance chains that distinguish original, append, override, and effective values when the active BitBake server supplies them.

## Explicit configuration controls

- [NOT_STARTED] Add a read-only BBMASK view first, populated by BitBake's effective configuration and provenance.
- [NOT_STARTED] Add an opt-in BBMASK editing dialog that previews the exact `conf/local.conf` change, writes only after confirmation, and refreshes BitBake's effective configuration afterward.
- [NOT_STARTED] Add equivalent preview/confirmation workflows for supported Devtool and layer operations; Yoctui must never silently modify configuration.

## Verification

- [NOT_STARTED] Validate these workflows against a supported, real Yocto/BitBake environment and record the compatibility matrix.
