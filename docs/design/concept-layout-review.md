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
