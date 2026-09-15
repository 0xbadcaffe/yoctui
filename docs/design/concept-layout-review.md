# Concept layout review

Checkpoint: `v0.1.102` (`3edb883`). Original M21 concepts remain unchanged.
The revised production renderer uses real typed state at 160x50 and responsive
neighboring sizes. PNGs are deterministic cell captures, not live-build evidence.

| Scene | Production layout and reference adaptation |
| --- | --- |
| [Dashboard](m22/production-raster/01-idle-dashboard.png) | Overview cards, retained build/job history, semicircular capacity dials, quick actions and Project Inspector. Missing sstate remains unavailable. |
| [Active tasks](m22/production-raster/02-active-build-tasks.png) | Build progress/table, correlated log, history and telemetry; one inspector contains task facts, paths, actions and system status. |
| [Errors](m22/production-raster/03-failed-build-errors.png) | Failure summary and table above correlated output; recovery and filters below; readable clock times and a compact fact/action inspector. |
| [Rootfs](m22/production-raster/04-rootfs-composition.png) | Braille pie paired with exact bytes, package selection and retained filesystem context; visible tab names and semantic colors. |
| [Editor/menu](m22/production-raster/05-editor-application-menu.png) | File tree and inspector span the editor height, diagnostics/diff stay under the document, and the F10 menu owns focus. |
| [Terminal sessions](m22/production-raster/06-terminal-sessions.png) | Visible session tabs, two bounded PTYs, writer/read-only status, search/history and a prefix-help rail. |

Terminal cells approximate the mockup's curves, border placement and text
spacing. Extra real destinations and filesystem context remain available. No
mockup timestamp, progress value, path or username enters production state.
Fixture telemetry is explicit test data; terminal samples use neutral prompts.

Renderer v2 draws Unicode Braille, basic box borders and block elements directly
from cell symbols because the pinned text font lacks Braille. It changes no
application values. Regression tests cover all 256 Braille masks, continuous
border endpoints, filled blocks, region geometry and resize/mouse boundaries.
The original PNG hashes and historical live capture records remain unchanged.

Final v0.1.106 review covers all six linked PNGs. The workspace test suite passes
1,613 Rust tests (four existing ignored) and 52 bridge tests; strict workspace
Clippy and formatting pass. Six concept rasters and ten README rasters reproduce
exactly. The final version refresh changes only header cells in the cell goldens.
The retained real-Poky performance manifest predates source changes (first digest
mismatch: `crates/yoctui-app/src/environment_setup.rs`); this delivery makes no
fresh live-performance certification.

## Added-value review and next sequence

The September 2026 review compares all six original PNGs with the current
production captures and ranks workflow value rather than pixel similarity.

| Concept | Added value | Recommendation |
| --- | --- | --- |
| Dashboard | High: one glance combines build state, history, resource pressure and next actions. | Keep. The meter stroke, value placement and context now match the supplied instrument reference; unavailable sstate remains explicit. |
| Active tasks | High: progress, task identity, correlated output and retained history support live build supervision. | Keep the present production composition. Polish only measured log-navigation or task-selection gaps. |
| Failed errors | High: failure selection, correlated log search, filters and recovery actions shorten diagnosis. | Keep. Prioritize action clarity and source correlation over more decoration. |
| Rootfs composition | High: the chart gives proportion while the table and package tree preserve exact authority. | Keep the production chart/table/tree combination; add detail only when package evidence supports it. |
| Editor and application menu | Highest remaining value: the recipe tree, large editor, validation/diff split and focus-trapped F10 groups form one complete editing workflow. | First future fidelity target. Expand usable editor space, keep diagnostics and diff visible, and retain disabled reasons and keyboard focus inside the menu. |
| Terminal sessions | High: tabs, split PTYs, writer ownership, search and prefix help make long-lived sessions manageable. | Keep. Native kernel and U-Boot menuconfig already use the full workspace; refine only from real PTY evidence. |

Recommended delivery order is editor/menu composition first, then focused
Active Tasks and Failed Errors workflow polish. Rootfs and Terminal Sessions
need evidence-led refinements rather than broad redesign. Concept-only sample
paths, usernames, timestamps, percentages and synthetic activity add no product
value and must not enter runtime state.

Version 0.1.111 applies the Dashboard meter decision to the production renderer
and synchronized cell/raster evidence. The original M21 concept PNGs remain
unchanged.
