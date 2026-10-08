# Interface behavior

Use the [user guide](user-guide.md) and [shortcuts](keyboard-shortcuts.md) for controls;
this guide defines shared behavior.

## Layout and focus

Header shows identity/status/telemetry; Navigator chooses destinations; the main
pane shows content; Footer shows controls. Inspector defaults off: Alt+i toggles
this session; Preferences saves startup visibility. Hidden panes cannot take focus.

One pane/dialog/editor/terminal owns input. Tab visits visible targets; Esc closes
the innermost owner. Stable IDs preserve selection through refresh/filter/resize;
zoom restores selection, scroll, follow, and subfocus. Navigation keeps work running.

## Menus and commands

F12: Workspace, Build, Actions, Navigate, Config, View, Devtool, Tools, Help.
Setup comes first; BBMASK is in Config; Preferences is first in View; Navigate
covers every destination. Arrows move, typing selects a prefix, Enter dispatches,
Esc closes; j/k navigate only without a typed prefix. All command/input routes
project one catalog and preserve disabled reasons and review requirements.

## Workbench usability contract

Views show title, selection, state, and actions. Empty/loading/unavailable/partial/
failed/cancelled/timed-out/lost states differ; missing is not zero. Identity/state
columns precede detail. Charts pair exact text; narrow, ASCII, no-color,
high-contrast, accessible-chart, and reduced-motion modes retain facts/actions.

| Family | Information |
| --- | --- |
| Dashboard / Tasks / Logs / Errors / History | Build status, output, diagnostics. |
| Layers / Recipes / Devtool / GitUI | Sources, metadata, edits, managed terminals. |
| Configuration / BBMASK / Signatures / Build History | Provenance, edits, comparisons. |
| Dependencies / Layer Relationships | Reported edges, cycles, partial results. |
| Images / Packages | Artifacts, packages, rootfs, installed services/rules. |
| Kernel / U-Boot / BIOS / Hardware | Provider/project files, debugging, viewers. |
| SDK / QEMU / Wic / Testing / Security / QA / Maintenance | Reviewed capability-aware jobs. |
| Terminal Sessions / Insights | Persistent terminals and typed summaries. |

## Builds, forms, and editors

Progress uses aggregates and observed timing; unknown totals/durations stay absent.
Build success cannot turn Lost tasks into successes. Log navigation pauses follow;
loss/truncation is independent of outcome. Dialogs trap input; confirmation
revalidates authority. Editors provide modes/search/undo/diff/conflict-checked saves;
single-line fields reject newline/control injection. Consequential operations
retain exact review.

## Terminals and viewers

Writers receive ordinary keys; Ctrl+B introduces Yoctui controls. Focus does not
grant ownership. Closing/detaching keeps processes; multiline paste and termination
require confirmation. Native render/mouse/resize geometry agrees. PDFs need graphics
and explain missing tools; XTerm handoff adapts to the parent terminal.

## Preferences and freshness

Preferences shows current/default/custom values; Backspace resets one, Alt+r all.
Save failures expose retry. Binding validation rejects collisions, ambiguous
prefixes, reserved routes, and removal of critical navigation. Launch-only flags
preserve saved color. Daemon build/instance/generation capability authority gates
results/actions; client PATH and stale results cannot enable them.

## Rendering and validation

Input/resize is independent of dirty-frame cadence: normally 4 Hz; saturated
cosmetics/clock 1 Hz. Hidden activity does not redraw. Compact telemetry retains
numeric CPU/RAM/filesystem values. Tests cover 80x24, larger/below-minimum sizes,
resize, themes, and ownership. [Six production scenes](design/README.md#ui-regression-scenes)
have exact cell/style/raster checks; fixtures do not certify live Yocto/boot.
