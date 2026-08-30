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
- a fresh representative release flamegraph with resolved application stacks
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

## M16 — Literal reference workbench

Goal: replace the earlier broad interpretation of the approved concept with a
measurable terminal-cell contract whose core visual composition matches the
reference while all values remain typed and authoritative.

Capabilities:

- strict default-theme `160x48` cell/style golden
- reference-proportioned header, mixed project Navigator, Tasks cockpit,
  Inspector stack, and stable F-key command rail
- intuitive pane focus and a directly usable persistent theme picker
- responsive degradation without panics or hidden destinations
- live Poky validation using the same rendering path as the deterministic gate

Exit criteria:

- every application-controlled cell in the canonical scene matches the reviewed golden
- the reference fixture cannot leak illustrative values into production
- every displayed function key invokes its labeled action
- theme and pane-focus behavior pass reducer, UI, and PTY interaction tests
- the complete workspace and live Poky workbench gates pass

## M17 — Responsive reference command rail

Goal: keep the reference's global F1–F10 navigation visible throughout every
wide workbench instead of tying it to one exact terminal width and screen.

M19 `FOOTER-UI-001` intentionally supersedes the fixed presentation while
preserving every typed function-key route. The footer is now contextual and
bounded; the complete truthful F1–F10 catalog remains in Help, and the
canonical footer geometry remains unchanged.

Capabilities:

- stable F1–F10 rail on every screen at 130 columns or wider
- exact canonical 160×48 Tasks footer geometry remains unchanged
- contextual action footer remains available below the wide breakpoint
- installed release and PTY regression validation

Exit criteria:

- 130-, 160-, 180-, and 200-column TestBackend scenes expose the global rail
- Dashboard and Tasks both expose all ten function-key labels when wide
- compact layouts retain contextual shortcuts without horizontal panics
- the shell-resolved release binary matches the verified local artifact

## M18 — Yocto release capability compatibility

Goal: make Yoctui functionality follow the authoritative capabilities of the
connected Yocto/OpenEmbedded/BitBake environment instead of assumptions made
from the installed Yoctui binary.

Capabilities:

- authoritative typed environment identity with explicit unknown fields
- one behavior-oriented capability catalog and snapshot with evidence/reasons
- safe direct probes plus centralized conservative version fallbacks
- daemon-owned, generation-correlated capability state shared by every client
- compatible BitBake/API/utility implementations selected before typed argv
- independently probed Devtool and Recipetool subcommands/options with exact
  unavailable reasons and no cross-command authorization
- independent bitbake-layers read, create, option, add, and remove capabilities
- complete typed utility-family classification and exact command authority;
  host PATH never proves build-environment compatibility
- complete workspace action inventory, pure availability/revalidation policy,
  and shared client/runtime enforcement
- dynamic workspace/UI gating and an Environment/Compatibility inspector
- deterministic release-generation fixtures and current live multi-release evidence
- offline evidence validation plus isolated scheduled/manual fresh official
  older/latest runs with retained role-scoped diagnostics

The UI delivery is dependency-ordered: first one typed presentation projection,
then the responsive Environment/Compatibility workspace, then visible action
gating across every existing workspace and dialog. The parent UI gate closes
only after all three pass their focused and terminal-snapshot checks.

Exit criteria:

- direct capability evidence is preferred over release-number assumptions
- renderers and workspaces contain no scattered release/version policy
- older supported environments preserve safe workflows and explain unavailable ones
- unknown future releases expose only positively evidenced functionality
- latest supported stable and a materially older release have current live evidence
- exact machine-readable evidence records expire after 90 days or a relevant
  capability-contract change and never convert fixture/development runs into claims
- fixture-only tests cannot satisfy a live compatibility claim
- `./scripts/verify-compatibility.sh` independently enforces the milestone

## M19 — Next-Generation TUI Implementation and Polish

The requested milestone name used `M13`, but `M13 — Dense terminal workbench
redesign` is an existing completed and evidence-backed milestone. This work is
therefore registered as M19 without renumbering or rewriting historical tasks.
The requested parent task ID remains `M13-UI-001` for traceability.

Goal: evolve the literal workbench into a polished terminal-native IDE while
keeping Navigator / Workspace / Inspector architecture, typed actions and
effects, bounded state, capability-correlated availability, and real data as
the only rendering authority.

Capabilities:

- documented wide, medium, narrow, and below-minimum layout behavior
- reusable pane, section, status, empty/loading/unavailable, scroll, and
  responsive-column primitives
- complete semantic theme roles with high-contrast and no-color behavior
- adaptive Tasks, Logs, Jobs, Inspector, header, footer, search, palette, and
  dialog presentation
- provenance-audited bounded telemetry with honest unavailable states
- keyboard/mouse parity, reduced motion, terminal-reader text equivalents,
  PTY coverage, and explicit breakpoint tests
- semantic TestBackend snapshots, a small reviewed target-design golden set,
  and style invariants
- measured rendering budgets and evidence-backed caching only where justified
- fresh real-Poky UI evidence and current real-binary README screenshots

Exit criteria:

- every required M19 task is `DONE` with its focused evidence
- no displayed value or action is fabricated from the concept image
- all existing workspace functionality and capability gating remains reachable
- `scripts/verify-next-generation-ui.sh` independently verifies the requested
  unit, visual, golden, keymap, responsive, mouse, PTY, accessibility,
  performance, live-evidence, screenshot, Clippy, formatting, and regression
  categories
- `./scripts/verify-completion.sh` passes without weakening an older gate

## M20 — Raw BitBake Command Workbench

Goal: provide expert users with a structured, capability-correlated browser
over the BitBake CLI command surface without introducing a shell-evaluation
escape path or duplicating the embedded terminal.

Capabilities:

- a tracked Wrynose 6.0 / BitBake 2.18 reference snapshot with exact catalog
  traceability
- typed categories, command templates, descriptions, parameters, interaction
  modes, safety classes, and capability requirements
- authoritative recipe, image, target, task, and multiconfig selection with
  bounded manual entry where BitBake accepts it
- a bounded expert argument editor and exact indexed native-argv preview
- daemon-owned noninteractive jobs and daemon-owned interactive PTY sessions
- responsive category, command, help, configuration, output, history, search,
  and favorite workflows
- atomic persistent favorites and bounded command history
- dynamic availability from the connected daemon capability snapshot
- mouse, keyboard, accessibility, security, fixture, and live-BitBake evidence

Exit criteria:

- reference-only, companion-tool, pipeline, and conceptual examples are never
  misrepresented as executable Raw Mode commands
- ordinary execution never uses `sh -c`, `bash -c`, `eval`, or an equivalent
  command string
- unavailable or stale capability state fails before process or PTY creation
- closing or detaching a view does not implicitly terminate daemon-owned work
- `./scripts/verify-raw-mode.sh` and the unchanged full completion gate pass

## M21 — One-Stop Yocto Workbench Usability

Goal: turn the complete typed workbench into the most discoverable, consistent,
beautiful, and efficient terminal environment for daily Yocto work without
trading away authoritative data, safety, accessibility, or bounded behavior.

The detailed research, widget decisions, interaction contract, license policy,
phase progress, test matrix, and completion criteria live in
[`workbench-ux-roadmap.md`](workbench-ux-roadmap.md).

Capabilities:

- one typed action catalog shared by application/context menus, palette, Help,
  footer, mouse routes, keybinding preferences, and tests
- stable mnemonic defaults, scoped configurable bindings, collision detection,
  contextual discovery, predictable focus/subfocus/zoom, and common scrolling
- authoritative hierarchical progress, resource/cache meters, throbbers,
  telemetry histories, charts, accessible checkboxes, and consistent state text
- virtualized searchable logs, a safe reducer-owned multiline editor, trees,
  scroll views, variable-height lists, and dependency topology
- image-correlated package and filesystem rootfs composition with pie/bar/table/
  tree views and exact accessible fallbacks
- a first-class daemon-owned terminal/session workspace, with a measured
  `tui-term` compatibility decision that cannot weaken the typed screen boundary
- capability-aware command center, onboarding, preferences, responsive layouts,
  accessibility, performance, real-PTY tests, and supported live-Yocto evidence
- license/MSRV/source/feature review, notices, SBOM, locked dependencies, and
  `cargo deny` for every adopted third-party widget

Exit criteria:

- all 38 required M21 tasks are `DONE`
- menus, Help, palette, footer, and configured bindings cannot drift
- all visual progress and composition values are typed and text-equivalent
- terminal, editor, rootfs, log, focus, scroll, mouse, and keyboard flows pass
  deterministic and real-PTY coverage at every supported breakpoint
- every dependency has current compatible license and supply-chain evidence
- the expanded performance matrix stays below the existing 10 ms/frame ceiling
- supported live-Yocto evidence and user documentation are current
- `./scripts/verify-workbench-ux.sh` and the unchanged completion gate pass

## M22 — Concept-to-Live UI Parity

Goal: make every reviewed concept use case reproducible through the production
Yoctui renderer and demonstrably reachable in a real supported-host instance,
without treating generated artwork or broad scenario labels as implementation
evidence.

Capabilities:

- machine-checked per-scenario feature, fixture, raster, and live-evidence contracts
- a complete failed-build workspace with summary, structured diagnostics,
  correlated paused log, textual filters, and recovery actions
- canonical-width Rootfs composition with chart, exact table, accessible batch
  selection, and filesystem drill-down visible together
- a real recipe editor and focus-trapped F10 application menu composition
- live daemon-owned Terminal Sessions navigation, split, writer/read-only, and
  prefix-help evidence
- deterministic PNG rendering from exact production TestBackend cells and styles
- supported-host live captures attributed only to interactions the harness drove

Exit criteria:

- every scenario manifest gap is closed by a `DONE` owner task
- deterministic cell/style goldens and app-derived raster captures agree
- the live harness drives and verifies each claimed screen instead of inferring it
- the concept comparison report records fixture, raster, and live results separately
- `./scripts/verify-m22-concept-parity.sh` and the unchanged completion gate pass

## M23 — Integrated Devtool Editing and Shell Workflow

Goal: make the selected-recipe development loop explicit and continuous inside
Yoctui: prepare a Devtool workspace, edit metadata or source with useful
language context, build the owning recipe, and publish the change back to a
configured layer without losing terminal or job state.

Capabilities:

- a shared reducer-owned workspace editor for recipes and Devtool source trees
  with cursor motion, selection, undo/redo, search, diff/save state, line
  numbers, language detection, syntax presentation, and bounded diagnostics
- direct continuation from `devtool modify` into edit, selected-recipe build,
  `update-recipe`, and configured-layer `finish`
- user-visible Devtool workspace and `edit-recipe` session routes
- a focus-trapped launch chooser before build shell, devshell, menuconfig, or
  Devtool interactive sessions start
- embedded daemon-owned PTY and detached desktop-terminal destinations built
  from the same validated native argv and initialized environment

Exit criteria:

- no interactive terminal is spawned before the user confirms its destination
- cancelling the chooser creates neither an embedded nor detached process
- the editor identifies BitBake metadata and common source languages without
  claiming LSP authority that is not present
- recipe builds target the exact Devtool recipe and patch publication remains
  capability- and configured-layer-gated
- focused model/app/UI/CLI tests, responsive rendering, strict lint, installed
  release smoke validation, and the unchanged completion gate pass

## M24 — Real Yoctui Design Regression Gallery

Goal: keep the six supported-host M22 Yoctui screens directly reviewable under
the design documentation and prevent their provenance, membership, ordering,
dimensions, or bytes from drifting away from the live evidence bundle.

Capabilities:

- one documented gallery containing idle, active build, failed build, rootfs,
  editor/menu, and terminal-session screens
- a machine-readable capture manifest with exact source commit, binary, host,
  Yocto/BitBake, machine, target, geometry, and per-screen SHA-256 identity
- byte-for-byte linkage from every design PNG to its supported-host live raster
- regression coverage for exact membership, ordering, README links, hashes, and
  `1600x1000` dimensions

Exit criteria:

- all six real Yoctui screens render from the design README
- no fixture, production-cell raster, or concept artwork can satisfy the live
  baseline checks
- `python3 scripts/test-m22-live-design-gallery.py`, documentation checks, and
  the M22 parity gate pass

## M25 — M21 Visual Resemblance Remediation

Goal: correct production workbench geometry and scene composition so the real
executable visibly follows the six M21 concepts, and replace monochrome
semantic screenshots with color- and style-faithful PTY evidence.

Capabilities:

- two-level workbench header, M21 pane proportions, semantic title color, and
  bordered footer across all six scenes
- concept-shaped Dashboard and integrated recipe editor/F10 menu compositions
- ANSI SGR-aware live terminal composition serialized as exact cell/style data
- deterministic rasters for review followed by six fresh release-binary live
  captures from the initialized Poky environment

Exit criteria:

- all six deterministic production rasters pass reviewed geometry and semantic
  tests and materially resemble their M21 counterparts
- the live capture pipeline preserves foreground, background, and bold styles
- six current-commit live captures replace historical evidence as the visual
  regression baseline; workflow-only anchors cannot satisfy the gate

## M26 — Visible release identity

Goal: make the exact running Yoctui revision legible to operators and prevent
unversioned repository changes.

Exit criteria:

- the persistent header shows `yoctui v<workspace-version>` at every supported width
- all workspace packages and internal path constraints share one version
- CI and the completion gate reject commits that do not increment that version

## M27 — Dashboard telemetry dial fidelity

Goal: make normal-height Dashboard telemetry read as compact instruments and
materially match the M21 resource-meter concepts without sacrificing truthful
typed values or responsive fallbacks.

Exit criteria:

- CPU, RAM, and Build FS use foreground-only semicircular dials when a strip
  cell has enough width and height
- every dial retains its exact percentage and metric-specific context in text
- Unicode, ASCII, no-color, reduced-motion, short-cell fallback, and
  unavailable-state behavior remain deterministic and tested
- production scenario goldens and design rasters encode the reviewed change

## M28 — Operator shell polish

Goal: make resource monitoring and everyday shell navigation denser, quieter,
safer, and more immediately legible without importing another application's
implementation or visual identity wholesale.

Exit criteria:

- CPU, RAM, and Build FS use original history-first monitor tiles with a live
  trend field and thin threshold meter, and narrow network/disk cells keep both
  current rates and retained history visible
- Left/Right navigate Navigator, Workspace, and Inspector; Navigator tree
  expansion remains available on h/l
- unknown Navigator/task state avoids repeated question-mark markers, while
  active tasks remain left-aligned with an explicit circular activity marker
- the header highlights project/target/machine/distro and identifies Local or
  SSH access with a validated remote client IP
- theme choices use color-oriented names without changing persisted identifiers
- q/Ctrl-C and active-build c require exact, focus-trapped confirmations
