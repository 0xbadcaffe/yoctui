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
- terminal restoration
- validated live BitBake compatibility

Exit criteria:

- real build smoke tests on supported versions
- normal completion, failure, cancellation, and bridge loss tested
- typed backend-to-model event contract enforced

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
