# Yoctui Product Roadmap

This roadmap defines the stable milestone sequence. Atomic implementation state lives in `docs/task-registry.toml`.

## Product completion rule

Yoctui is 100% complete only when:

- every required task in `docs/task-registry.toml` is `DONE`
- `./scripts/verify-completion.sh` passes
- the supported live Yocto/BitBake compatibility matrix has been validated
- no required workflow is represented only by a placeholder
- documentation matches the shipped behavior

## M0 — Governance and reliable execution

Goal: a fresh agent can continue implementation without inventing scope or losing progress.

Exit criteria:

- root `AGENTS.md`
- one active task in `docs/current-task.md`
- machine-readable task registry
- human-readable implementation status
- roadmap verification
- objective final completion gate
- architecture and UI specifications treated as contracts

## M1 — Reliable BitBake cockpit

Goal: reliably control and observe real builds.

Capabilities:

- workspace discovery
- bridge and process backends
- build start and cancellation
- parse and task lifecycle
- bounded logs
- structured errors
- build history
- CPU, memory, and disk telemetry
- responsive CPU/memory/disk gauges, bounded history sparklines, load averages,
  and honest average task velocity/ETA
- terminal restoration
- validated live BitBake compatibility

Exit criteria:

- real build smoke tests on supported versions
- normal completion, failure, cancellation, and bridge loss tested
- typed backend-to-model event contract enforced
- fractional process progress and PID-only task progress normalize without
  disconnecting the typed stream, and determinate task progress renders as bars
- daemon-owned builds restore and continuously update the same typed build and
  task cockpit after clients detach and reattach

## M2 — Persistent Yocto workbench

Goal: navigation remains useful while jobs run.

Capabilities:

- persistent shell
- responsive wide, medium, narrow, and too-small layouts
- shared focus router
- dialog stack
- command palette
- contextual footer
- themes and accessibility preferences
- notifications
- persistent background-job model
- Tasks, Logs, Errors, Settings, and Images workspaces

Exit criteria:

- all long operations survive workspace navigation
- all dialogs trap focus
- no terminal size causes a panic

## M10 — Optional project profiles

Goal: let teams record portable Yoctui intent in an optional
`.yoctui/project.toml` without changing vendor/layer metadata or replacing
BitBake authority. Profiles contain typed favorites, presets, and workflows;
they never execute arbitrary shell text when loaded, and personal settings
remain user-local.

The profile contract is versioned and fail-closed. References are portable and
repository-relative, stale authoritative identities remain explicit, loading
is inert, and generation is an explicit reviewed action. Real-Poky acceptance
must cover both an unmodified no-profile checkout and an explicitly generated
profile before compatibility is claimed.

## M11 — Persistent daemon and session architecture

Goal: provide one Rust-native, terminal-native Yoctui daemon that owns BitBake,
background work, and PTYs while attachable Ratatui clients render and control
that state. Local IPC is the default; SSH clients attach on the build host
without opening an unauthenticated TCP service. Client disconnect, SSH loss,
daemon restart, and host reboot semantics must be explicit and honest.

## M3 — Recipe, layer, metadata, and dependency development

Goal: complete daily recipe and layer work without leaving Yoctui except for intentional editor/terminal launches.

Capabilities:

- lazy layer tree
- file preview and editing
- Git decorations and refresh
- recipe search and actions
- configuration provenance
- Devtool lifecycle
- task and recipe dependency exploration
- signature inspection
- package-data browser
- recipetool workflows
- bitbake-layers diagnostics

## M4 — Images, packages, SDK, QEMU, and Wic

Goal: build, inspect, run, and deploy images and SDKs.

Capabilities:

- image artifacts
- package membership
- SDK generation and publication
- managed QEMU sessions
- Wic creation
- protected device writing
- native tool and extracted-SDK workflows

## M5 — Testing, QA, CVE, and SPDX

Goal: make validation and security workflows first-class.

Capabilities:

- oe-selftest
- bitbake-selftest
- testimage
- testsdk
- ptest
- resulttool
- typed result regression comparison and JUnit export
- CVE analysis
- SPDX/SBOM
- kernel config checks
- URI, patch, and license QA
- layer QA

## M6 — Maintenance and release engineering

Goal: safely expose advanced maintenance.

Capabilities:

- sstate readiness and cleanup
- PR service diagnostics and tools
- hash server diagnostics
- locked signature generation
- build comparison
- Git archive
- optional pull-request workflows
- repo manifest integration
- Toaster detection

## M7 — Production hardening

Goal: release-quality reliability.

Exit criteria:

- formatting, lint, tests, coverage, audit, and deny pass
- property, fuzz, stress, terminal, and process-tree tests
- deterministic profiling and memory reports
- complete compatibility matrix
- installation and operator documentation
- final completion gate passes from a fresh checkout

## M8 — In-app build environment onboarding

Goal: make first-run BitBake setup safe and understandable without requiring
shell setup before starting Yoctui.

Capabilities:

- launch without a build-directory argument
- typed existing-source and reviewed Poky-clone profiles
- validated environment initialization and interactive setup-shell handoff
- managed child-only environment capture
- explicit BitBake connection verification before build controls unlock

Exit criteria:

- startup never treats the current directory as an implicit build
- clone, initialize, cancel, shell, and verification outcomes remain distinct
- no build or metadata action starts before a typed connection succeeds

## M12 — crates.io distribution

Goal: ship the first installable public release as `yoctui` 0.1.0.

Capabilities:

- self-contained bridge behavior after `cargo install yoctui`
- complete crates.io metadata and bounded package contents
- publishable internal dependency crates with private test/support crates excluded
- dependency-ordered, reproducible publication and clean-install smoke validation

Exit criteria:

- the packaged binary does not depend on repository-relative runtime files
- every public package passes packaging and isolated build checks
- `yoctui` 0.1.0 is published under the intended crates.io account
- a clean registry installation runs the published binary successfully

## M13 — Dense terminal workbench redesign

Goal: make the persistent Ratatui client match the approved compact IDE-style
Yocto operations workbench while preserving typed behavior and accessibility.

Capabilities:

- one-line project and daemon/BitBake status header
- grouped IDE-style Navigator with full-row selection
- compact panel chrome and contextual command rail
- three-tier Tasks cockpit with live log and retained job history
- structured task Inspector with context actions and system status
- wide, medium, narrow, no-color, and reduced-height regression coverage

Exit criteria:

- the default dark workbench preserves the approved blue/lime/amber visual hierarchy
- all displayed values come from typed model or daemon replica state
- keyboard, mouse, focus, theme, and responsive contracts remain intact
- deterministic TestBackend and PTY snapshot checks pass

## M14 — Live workspace usability recovery

Goal: make the first real Poky launch reliably show the approved workbench and
metadata without hidden state from tests or prior diagnostic invocations.

Capabilities:

- launch-scoped backend and no-color overrides
- isolated PTY/snapshot configuration and runtime state
- metadata-capable local bridge fallback when the daemon is absent
- non-obscuring daemon-disconnected status
- directly discoverable theme selection and explicit pane-focus routing
- live Poky metadata and PTY visual acceptance

Exit criteria:

- a normal launch against `~/src/poky/build` shows workspace, layers, and recipes
- snapshot tests cannot alter the operator's session file
- selecting a theme visibly changes the complete colored shell
- the footer names current, next, and previous focus destinations
- live and deterministic recovery verification passes

## M15 — Clean installed startup diagnostics

Goal: make the executable selected by the user's shell start the real Poky
workbench without terminal contamination and keep theme selection verifiable.

Capabilities:

- bounded bridge stderr capture outside the alternate screen
- actionable bridge failures with retained diagnostic context
- live installed-binary startup and theme-picker acceptance
- explicit local-development reinstall guidance for a published version

Exit criteria:

- BitBake startup notes and warnings never appear outside Ratatui panels
- bridge startup failures retain a bounded diagnostic tail
- `Ctrl+P` → `Choose theme` changes and persists the theme in a real PTY
- the shell-resolved executable matches the locally verified release binary
