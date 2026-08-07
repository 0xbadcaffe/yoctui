# Yoctui release-quality UI acceptance contract

This is the executable contract for release validation. Tests must drive the
release binary through a real PTY where a task calls for PTY evidence. A
fixture, Ratatui `TestBackend`, or fake process may prove deterministic model
or rendering behavior, but must be labelled as fixture evidence and never be
reported as live Yocto compatibility.

## Supported terminal matrix

The harness must run every applicable scenario at 80x24 (narrow), 100x30
(medium), 160x48 (wide), and the minimum supported size 40x12. It must also
exercise resize transitions wide→medium→narrow→too-small→wide. Too-small
renders a safe explanatory state, never panics, and preserves the active job
and return focus.

## Screen and navigation contract

The persistent shell contains Header, Navigator, Workspace, Inspector, and
Footer. Acceptance coverage visits Dashboard, Layers, Recipes, Tasks, Logs,
Errors, Configuration, Images, Packages, SDK, Testing, Security, QA,
Dependencies, Devtool, Settings, Maintenance, Help, and every documented
child workspace (QEMU/Wic, signatures, package details, test results, and
maintenance dialogs). Each destination must expose a stable title/anchor and
the selected identity must survive refresh, resize, and background-job output.

Navigator → Workspace → Inspector is the canonical traversal. `Tab` advances
and `Shift+Tab` reverses through the currently visible regions; narrow layouts
use the visible-pane switcher. Inspector overlays and child workspaces restore
the exact prior pane on `Esc` or completion. Background jobs remain visible
and cancellable while navigating.

## Focus, dialogs, and safety

Dialogs trap focus. `Tab`/`Shift+Tab` visit only dialog fields and actions;
pane shortcuts are inert until the dialog closes. `Enter` activates the
primary action, `Space` toggles the focused choice, and `Esc` cancels or
returns outward. Destructive, network, device, overwrite, and root-sensitive
operations require an exact preview followed by explicit confirmation;
cancellation leaves state and history intact. Multiline paste is confirmed.

## Keyboard acceptance matrix

The PTY keymap test is generated from the typed input and command catalogs and
must cover every entry below, including an invalid-context assertion where the
entry is disabled:

| Scope | Keys and required result |
| --- | --- |
| Global | `?` Help; `F5` build; `Ctrl+P` command palette; `/` search; `Tab`/`Shift+Tab` focus; `Esc` dashboard/out; `q` quit confirmation; `Ctrl+C` cancel; arrows and `j/k` selection; `Enter` activate; `Backspace` edit/delete where offered |
| Navigator | `j/k` or `Up/Down` move; `Enter` open; `Tab` workspace; `Shift+Tab` inspector |
| Layers/files | `Right/l` expand, `Left/h` collapse, `Enter` open/toggle, `e` edit, `o` external editor, `R` relationships, `r` refresh, `.` hidden files, `/` search, `g` Git, `m` metadata, `d` dependencies |
| Tasks | `f` state filter, `F` field filter, `/` edit filter, `d` duration filter, `c` cancel |
| Logs/errors | `f` follow, `w` wrap, `n/N` next/previous match, `s/R/T/B` filters, `o` source, `C` copy |
| Recipes/config/dependencies | `b` build, `i` image, `r` refresh/recipes, `v` configuration, `o` provider, `L` task log, `1/2` comparison sides, `c` compare, `e` editor |
| Images/QEMU/Wic/SDK | `Q` launch QEMU, `x` cancel session, `D` protected device write, `w` Wic, `s` SDK, `i` image selection |
| Dialog/editor | `Tab`, `Shift+Tab`, arrows, `Space`, `Enter`, `Esc`, `Backspace`, `Ctrl+S` save, `Ctrl+B` build saved recipe, `Ctrl+C` cancel |
| Palette/help/settings | printable search, arrows, `Enter` dispatch, `Esc` close, `r` retry settings save, `Space`/arrows change values |

The help screen and contextual footer are the oracle for labels. Any active
shortcut absent from the typed catalog or any documented shortcut without an
executable path fails the release gate.

## Evidence and artifact policy

Every PTY scenario records raw ANSI input/output, parsed terminal cells, final
dimensions, semantic screen text, process/job logs, and exit status. Failures
also retain a terminal capture and a rendered text/PNG or HTML diff when a
visual assertion applies. Artifacts are bounded, deterministic, and stored
under `artifacts/release-quality/` with the scenario, commit, host, and
fixture/live label. Secrets and environment tokens must be redacted.

## Live Yocto policy

Fresh-Poky and compatibility scenarios must record the exact Poky revision,
BitBake version, host image, machine, build directory, command, and outcome.
Only those records may claim live compatibility. Deterministic fixtures and
fake bridges are valid for control-flow coverage only. The required live
workflow is a clean checkout, `oe-init-build-env`, qemux86-64, doctor,
workspace/layer/recipe/configuration inspection, `core-image-minimal` start,
progress observation, cancellation, and a bounded successful completion (or
an explicitly documented upstream/environment blocker).

## Exit criteria

This contract is complete when it is non-empty, referenced by the active task,
and `./scripts/verify-roadmap.sh` passes. Subsequent tasks add executable
evidence; they must not weaken these requirements or the distinction between
live and fixture results.
