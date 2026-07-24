# Yoctui implementation status

Status values: `NOT_STARTED`, `IN_PROGRESS`, `BLOCKED`, `DONE`.

## Authoritative product contracts

- [IN_PROGRESS] Follow `docs/ui-spec.md` as the authoritative UI, navigation, focus, dialog, theme, animation, and keyboard-shortcut contract. Verification: UI contract tests plus `./scripts/verify-ui-spec.sh`. The agent must update `docs/ui-spec.md` in the same commit as any intentional UI behavior change.
- [NOT_STARTED] Refactor disconnected screens into the persistent Navigator / Workspace / Inspector shell defined by `docs/ui-spec.md`. Verification: Ratatui `TestBackend` layout tests at wide, medium, narrow, and too-small terminal sizes.
- [NOT_STARTED] Implement the shared focus router and dialog stack. Verification: keyboard navigation tests for `Tab`, `Shift+Tab`, `Esc`, modal focus trapping, and context-specific actions.
- [NOT_STARTED] Implement the shared context-sensitive footer shortcut bar. Verification: screen/focus/dialog shortcut snapshot or semantic rendering tests.
- [NOT_STARTED] Implement theme and preference infrastructure with built-in `dark`, `light`, `matrix-green`, `high-contrast`, and `monochrome` themes. Verification: configuration tests and semantic color-role rendering tests.
- [NOT_STARTED] Implement configurable fast task animations and a reduced-motion mode without implying false numeric progress. Verification: deterministic animation-frame tests and redraw-rate benchmarks.

## Foundation and naming

- [DONE] Rust workspace exists and dependency direction is acyclic. Verification: `cargo metadata --no-deps`. Commit: `43ca39b`.
- [DONE] Remove every obsolete legacy application name from crate names, directories, imports, tests, paths, scripts, and history-facing checks. Verification: `./scripts/check-obsolete-name.sh`. Commit: `d2c38ad`.
- [DONE] Public binary, configuration directory, environment prefix, bridge name, and UI branding use Yoctui. Verification: `cargo run -p yoctui -- --help`. Commit: `ad603ad`.
- [IN_PROGRESS] Add repository lint/format configuration and a fresh-clone setup check. Verification: `./scripts/check-checkout.sh`; editor configuration and hidden-path naming guard: `1eab5bf`. Final completion gate remains pending.

## Application and terminal

- [DONE] Model, typed actions, pure reducer, bounded logs, task state, and basic TUI screens exist. Verification: `cargo test -p yoctui-model -p yoctui-ui`. Commit: multiple pre-guide commits.
- [DONE] Terminal guard restores raw mode, alternate screen, cursor, mouse, bracketed paste, and panic state. Verification: Rust tests and manual pseudo-terminal test. Commit: `7d50f94`.
- [IN_PROGRESS] Handle resize, supported termination signals, terminal restoration in a pseudo-terminal, and dynamic unavailable-command help. Verification: `./scripts/test-terminal.sh` and `cargo test -p yoctui`. Pseudo-terminal and SIGTERM coverage commits: `df411ad`; current signal commit pending.
- [IN_PROGRESS] Complete dashboard metrics, build dialog, confirmations, notifications, and backend-driven TUI effects. Backend, status, task counts, diagnostics, active tasks, and recent output: `dd1f6d5`; validated build-target dialog: `a7e89b0`; fake bridge build integration: `bb5dd0d`; parse progress: `7bdd355`; backend exit code: `e9bf2f7`; stale state reset before a new build: `59ee5df`; active package progress gauges: `59117fe`; inherited shell and machine-aware build options: `0a29874`; live CPU and build-filesystem free-space telemetry: `e705c8f`; bounded completed package-task history: `b40d874`; active Yocto header and screen-aware footer shortcuts: `13dd151`; in-session completed-build history: `076ae78`. Notifications, shared shell migration, animated task bars, and build-state edge cases remain.

## Process backend

- [DONE] Process output capture, ANSI stripping, severity classification, process-group cancellation, escalation, invalid UTF-8 handling, and fake-process tests exist. Verification: `cargo test -p yoctui-bitbake`. Commit: `c477b6a`, `6c53488`, `491db9f`.
- [IN_PROGRESS] Bound individual process lines, preserve multiline diagnostics, map exit status, test forced cleanup/child trees/high-volume output, and expose cancellation outcome. Verification: process integration tests. Bounded lines: `ecbe553`; exit-code commit pending.

## Protocol and bridge

- [DONE] Versioned envelopes, sequence/correlation fields, NDJSON framing, bounded transport reads, malformed/oversized handling, and Python framing tests exist. Verification: protocol and bridge tests. Commit: `3730f35`, `78cc988`.
- [IN_PROGRESS] Implement bridge handshake negotiation, graceful shutdown command, compatibility adapters, mocked BitBake integration boundary, and typed workspace/recipe/layer/variable/dependency responses. Verification: pytest with mocked modules and `cargo test -p yoctui-bitbake`. Handshake: `496b177`; shutdown acknowledgement and child exit: `ad52654`; typed responses: `0f4bb33`; adapter selection: `c5daf0b`; mocked event normalization: `da142a2`; authoritative dependency query: `9ffe75d`.
- [IN_PROGRESS] Connect bridge to a supported live BitBake server, normalize native events, start builds, request native cancellation, and document tested BitBake versions. Verification: mocked `bb.server` adapter tests; native-style task, parse, warning, and error normalization: `f2ca82f`, `7bdd355`, `82cbbf6`; server-backed variable query: `0fdf76c`; server-backed recipe/layer queries: `996a6d8`; server-backed workspace inspection: `e1ed404`; live-Yocto smoke workflow remains required. Server boundary and unavailable-server diagnostics: `993ac4c`.
- [IN_PROGRESS] Enforce the backend-to-UI typed-event contract from `docs/ui-spec.md`. Raw BitBake/process text must not be parsed in Ratatui widgets. Verification: architecture tests and UI tests using typed synthetic events only.

## Workspace, CLI, and configuration

- [DONE] CLI options, configuration precedence, headless inspection, doctor diagnostics, and read-only backend CLI commands exist. Verification: CLI tests and `yoctui doctor`. Commit: `e033f62`, `35fa2cb`, `1979825`.
- [IN_PROGRESS] Complete workspace fields, recipe/layer discovery, variable provenance, CLI subcommand outputs, editor configuration, session persistence, and all configuration settings. Environment-derived Yocto/OpenEmbedded release discovery: `0d693b9`; preferred editor configuration: `9ac79a6`; color configuration: `0c7e551`; cancellation timeout: `ad293af`; interactive session preferences and recent directories: `272c54c`; CLI variable provenance: `ee111c9`; server-backed workspace/recipe/layer data: `e1ed404`, `996a6d8`. Verification: fake bridge and CLI integration tests.
- [NOT_STARTED] Add UI preferences for theme, animation speed, reduced motion, pane sizes, compact header, shortcut footer visibility, mouse support, Unicode/icons, and remembered workspace. Verification: configuration precedence and persistence tests.

## Screens and interaction

- [IN_PROGRESS] Build the Yocto workbench: Devtool-backed in-TUI recipe editing with a two-pane workspace browser and save controls: `608b248`; active metadata layer in-TUI editing: `9fd6a2c`; BBMASK inspection: `2e459fd`; preview/confirmation-protected BBMASK local.conf edit and metadata refresh: `a04cb3e`; Devtool update-recipe: `43a1cbc`; destination-aware Devtool finish: `458aeea`; target-aware Devtool deploy-target: `795a758`; server-supplied recipe build/runtime dependency view and workspace recipe navigation: `1360412`, `6ba9d65`; server-supplied layer relationship view: `63946a0`, `bda2d8f`; task graph navigation remains planned in `yocto-workbench-plan.md`.
- [NOT_STARTED] Replace ad-hoc two-pane behavior with the lazy Layers tree and context-sensitive Inspector specified in `docs/ui-spec.md`. Verification: lazy expansion, file preview, editor refresh, Git status, and large-file tests.
- [IN_PROGRESS] Logs support bounded retention, pause/follow, wrap, severity filtering, notification display, vertical/horizontal navigation, and interactive text search with next/previous match navigation. Verification: model/UI tests. Commits: `26aad33`, `4017e02`, `c871b26`, `7c04b37`, `09c4978`, `7928bbd`, `6509c5d`; warning/error eviction indicators: `1359c28`.
- [IN_PROGRESS] Add recipe/task filters in UI, source-log/editor actions, and richer eviction detail. Recipe/task filter controls: `37a907c`; retained-byte/eviction detail: `2e8c4f1`; selected error source logs open in `$EDITOR`: `9a35b5d`.
- [IN_PROGRESS] Complete structured errors screen with selection/detail/log jump. Table/detail: `bec99cf`; selection: `4ed019b`; log jump: `8f0154f`; timestamp detail: `b14fcff`; expanded multiline detail: `4332055`. Cross-screen context and richer parsing remain.
- [IN_PROGRESS] Complete recipes screen with search/details/valid actions and destructive confirmations. Backend-loaded table, selection, and details: `58c9332`; case-insensitive metadata search: `494f289`; selected recipe build action: `b7b0564`; clean action: `44eab59`; cleansstate confirmation: `5672ce9`; menuconfig: `66dadbe`; selected recipe Devtool modify/editor workflow: `81011c6`; confirmation-protected Devtool reset: `987e434`.
- [IN_PROGRESS] Complete layers screen with metadata/search/open action. Backend-loaded table, selection, and metadata details: `63895c7`; case-insensitive metadata search: `494f289`; selected directory opening: `4039f66`; active-build highlighting: `0a29874`.
- [IN_PROGRESS] Complete read-only configuration screen with search, expansion, and provenance. Backend-loaded table, selection, and expanded values: `5677581`; case-insensitive metadata search: `494f289`; bridge-supplied provenance: `201819a`; provenance source opening: `d6686f4`; CLI provenance output: `ee111c9`; server-backed variable query: `0fdf76c`; live BitBake provenance remains.
- [NOT_STARTED] Add persistent background-job behavior so builds, QEMU, Wic, SDK, tests, and maintenance jobs continue while the user navigates Layers, Recipes, and other workspaces. Verification: reducer and integration tests.

## Yocto tool and workflow coverage

The following tool coverage is required. Each integration must use a typed action/dialog/workspace and must not degrade into an unstructured shell textbox as the main UX.

### Tier 1 — Core Yoctui

- [IN_PROGRESS] `bitbake`: build start, tasks, environment, task actions, signatures, dependency generation, cancellation, and build results.
- [IN_PROGRESS] `devtool`: modify, edit, build, update-recipe, finish, reset, deploy-target, workspace status, and Git integration.
- [IN_PROGRESS] `bitbake-layers`: layer listing, details, priorities, dependencies, overlays, appends, create-layer, and diagnostics.
- [DONE] `bitbake-getvar`: effective variable lookup and CLI output. Live provenance remains separately tracked.
- [IN_PROGRESS] `bitbake-diffsigs`: recipe/task signature comparison dialog and result viewer.
- [IN_PROGRESS] `bitbake-dumpsig`: signature dump viewer linked from tasks/recipes.
- [NOT_STARTED] `oe-depends-dot`: recipe/task dependency path navigation and “Why is this built?” workflow.
- [IN_PROGRESS] `oe-pkgdata-util`: package browser, file ownership, runtime dependencies, reverse dependencies, and image membership.
- [NOT_STARTED] `oe-check-sstate`: pre-build cache readiness, expected reuse, missing objects, and impact preview.
- [IN_PROGRESS] `recipetool`: create/append/newappend workflows with preview and confirmation.
- [NOT_STARTED] `runqemu`: managed QEMU dialog, background process, console/log view, and cancellation.
- [NOT_STARTED] `wic`: image creation dialog, kickstart preview, output inspection, and protected device-write workflow.
- [NOT_STARTED] `resulttool`: test result browser, regression comparison, metadata filters, and JUnit export.
- [IN_PROGRESS] `buildhistory-diff`: build comparison workspace and selected change details.

### Tier 2 — Development workbench

- [NOT_STARTED] `oe-run-native`: native tool selector and embedded/external terminal launch.
- [IN_PROGRESS] `oe-find-native-sysroot`: resolved sysroot display and command-copy workflow.
- [NOT_STARTED] `patchreview`: patch metadata, `Upstream-Status`, missing headers, filters, and editor actions.
- [IN_PROGRESS] `kas`: project detection, configuration summary, build invocation, and environment diagnostics.
- [IN_PROGRESS] Git integration: layer/source status, diffs, branches, commits, Devtool workspace changes, and patch export state.
- [NOT_STARTED] Repo manifest integration: repository revisions, dirty state, manifest projects, and layer-to-repository mapping.
- [NOT_STARTED] `oe-publish-sdk`: SDK artifact selection, publication dialog, checksums, and result display.
- [NOT_STARTED] `oe-debuginfod`: status, endpoint, symbol-path diagnostics, and configured debugger launch.
- [NOT_STARTED] `runqemu-extract-sdk`: extraction dialog and QEMU preparation workflow.
- [IN_PROGRESS] `cve-check-map-pkgs`: package mapping and CVE diagnostic integration.
- [NOT_STARTED] `yocto-check-layer`: layer QA execution and structured result viewer.

### Tier 3 — Testing and diagnostics

- [NOT_STARTED] `oe-selftest`: test selection, execution, progress, logs, and results.
- [NOT_STARTED] `bitbake-selftest`: test selection, execution, progress, logs, and results.
- [NOT_STARTED] `do_testimage` and `do_testimage_auto`: image test dialog and live result integration.
- [NOT_STARTED] `do_testsdk` and `do_testsdkext`: SDK test workflows and result integration.
- [NOT_STARTED] `ptest`: package test discovery, execution, and result display.
- [NOT_STARTED] `pybootchartgui`: build performance artifact discovery and viewer launch.
- [NOT_STARTED] `send-error-report`: preview, redaction, explicit confirmation, and submission result.

### Tier 4 — SDK, security, image, and QA tasks

- [NOT_STARTED] `do_populate_sdk` and `do_populate_sdk_ext`: SDK build dialogs and artifact browser.
- [NOT_STARTED] `do_create_spdx`: SPDX/SBOM generation and artifact viewer.
- [NOT_STARTED] `do_cve_check`: CVE run, summary, per-recipe detail, and report navigation.
- [NOT_STARTED] `do_kernel_configcheck`: kernel configuration QA and mismatch viewer.
- [NOT_STARTED] `do_menuconfig`, `do_diffconfig`, and `do_savedefconfig`: kernel/configuration workflows with external terminal/editor handling.
- [NOT_STARTED] `do_deploy`, `do_image`, and `do_image_complete`: artifact progress and deploy browser.
- [NOT_STARTED] `do_checkuri`, `do_patch`, and `do_populate_lic`: recipe QA actions and structured results.

### Tier 5 — Advanced maintenance

- [NOT_STARTED] `sstate-cache-management.sh`: impact preview, affected paths, estimated size, strong confirmation, and report.
- [NOT_STARTED] `bitbake-prserv-tool`: advanced PR service maintenance dialog.
- [NOT_STARTED] `gen-lockedsig-cache`: locked signature generation workflow.
- [NOT_STARTED] `oe-git-archive`: release engineering archive workflow.
- [NOT_STARTED] `build-compare`: build comparison workflow.
- [NOT_STARTED] `create-pull-request` and `send-pull-request`: optional release-maintainer workflow.
- [NOT_STARTED] `cve-check-map-pkgs`: advanced mapping/report workflow where not covered by the CVE workspace.

### Detect and diagnose, but do not normally expose as launchable tools

- [NOT_STARTED] `oe-init-build-env`, `oe-setup-builddir`, and `oe-buildenv-internal`: environment state detection and controlled wrapper launch.
- [NOT_STARTED] `bitbake-worker`: worker visibility and diagnostics only.
- [NOT_STARTED] `bitbake-prserv`: service status and connection diagnostics.
- [NOT_STARTED] `bitbake-hashserv`: service status and connection diagnostics.
- [NOT_STARTED] `runqemu-ifup`, `runqemu-ifdown`, and `runqemu-gen-tapdevs`: managed indirectly through QEMU networking dialogs.
- [NOT_STARTED] `yocto-layer`, `yocto-bsp`, and `yocto-kernel`: legacy compatibility detection with modern workflow recommendations.
- [NOT_STARTED] `toaster`: detect/configure/open optional web interface; do not duplicate its server internally.

## Reliability, testing, and quality

- [IN_PROGRESS] Expand model/protocol/UI/process tests; add fake bridge fixtures and integration test tree. Verification: `cargo test --workspace --all-features`, `pytest`, `./scripts/test-cli.sh`. Property tests: `b231871`, `36374dd`; bridge CLI smoke: `a1723c2`.
- [IN_PROGRESS] Add property tests, fuzz targets, stress/memory retention tests, benchmarks, and terminal integration tests. Retention and protocol framing properties complete; fuzz, stress, benchmarks remain.
- [DONE] Configure coverage (`cargo llvm-cov`, `pytest-cov`) with thresholds. Model/protocol Rust thresholds: `9c2ca20`; bridge Python coverage is 81.36%: `15ebffd`.
- [IN_PROGRESS] Configure audit/deny/ruff/mypy checks and complete CI matrix, optional real-Yocto, sanitizer, Valgrind, and flamegraph workflows. Ruff/mypy/pytest are enabled locally and in CI: pending commit; audit/deny and remaining workflows remain.
- [IN_PROGRESS] Run deterministic Valgrind, profiling, flamegraph, and memory workloads; commit concise reports. Reproducible bridge workload: `ed9ea9b`; release and Valgrind baselines: `664c36e`; Flamegraph remains pending.
- [IN_PROGRESS] Complete all documentation and compatibility matrix.
- [IN_PROGRESS] Add `scripts/verify-completion.sh`, artifacts directories, and completion-gate checks. Strict gate and artifact root: `2a623ef`; required coverage/audit/static-analysis tools and remaining product checks still prevent a passing final gate.
- [IN_PROGRESS] Add `scripts/verify-ui-spec.sh` to verify required workspaces, shared shell types, focus model, dialog types, theme names, footer shortcut system, and typed-event boundary. Initial shell/focus guard: pending commit.

## Agent execution rules

- [IN_PROGRESS] The agent must read `docs/ui-spec.md` and this file before implementation.
- [IN_PROGRESS] New user instructions override the current implementation queue. The agent must pause unrelated work, update the authoritative specification, implement the requested change, test it, commit it, and only then resume the checklist.
- [IN_PROGRESS] The agent must not continue unrelated implementation after a user changes the UI workflow.
- [IN_PROGRESS] The agent must not invent UI behavior outside `docs/ui-spec.md`.
- [IN_PROGRESS] Every intentional UI behavior change must update `docs/ui-spec.md` in the same commit.
- [IN_PROGRESS] After every coherent commit, the agent immediately continues with the next incomplete item until the completion gate passes.

## CONTINUE_FROM_HERE

Current phase: UI contract adoption and bridge reliability.

Immediate next item:

1. Add `docs/ui-spec.md` to the repository.
2. Refactor the application root into the persistent Header / Navigator / Workspace / Inspector / Footer shell.
3. Add shared focus routing and context-sensitive footer shortcuts.
4. Preserve current build execution and typed backend event behavior during the refactor.
5. Add `dark`, `light`, `matrix-green`, `high-contrast`, and `monochrome` theme roles.
6. Add deterministic animated live-task progress widgets with fast default animation and reduced-motion support.
7. Then resume the live BitBake server adapter work.

Relevant files:

- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `crates/yoctui-ui/`
- `crates/yoctui-app/`
- `crates/yoctui-model/`
- `crates/yoctui-bitbake/src/lib.rs`
- `bridge/yoctui_bridge.py`
- `bridge/tests/test_bridge.py`

Required first commands:

```bash
cat docs/ui-spec.md
cat docs/implementation-status.md
cargo test -p yoctui-ui -p yoctui-app -p yoctui-model
```

The agent must not skip directly back to unrelated backend work before the authoritative shell, focus, footer, themes, and task animation foundations are represented in code and tests.
