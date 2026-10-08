# Compatibility

**Yoctui functionality is Yocto-feature-correlated.** The selected initialized
build supplies tools/tasks/APIs; installing Yoctui does not install BitBake.
Use [Supported releases](supported-releases.md) for exact recorded revisions.

## Understanding unavailable actions

Open Compatibility or run `yoctui --build-dir "$BUILDDIR" doctor` (add --json
for structured output). Read the selected action's prerequisite before retrying.

| State | Meaning |
| --- | --- |
| Available | Required behavior detected. |
| Available with limitations | Working route with stated limits. |
| Unavailable | Required tool/task/artifact/environment absent. |
| Intentionally unsupported | Deliberately excluded. |
| Unknown | Missing, stale, incomplete, or conflicting evidence. |

## Detection

Daemon identity includes canonical build/source, tool/version, distro/machine,
layers, backend/protocol, and generation. Names/tags cannot invent a release;
client PATH cannot replace initialized-tool authority. Read-only probes/APIs and
metadata precede conservative version fallbacks; strong conflicts stay Unknown.
Changes invalidate caches/results. Tinfoil fallback: >=1.46,<2.0 legacy;
>=2.0,<2.19 modern; outside that range needs direct proof, not a release claim.

## Evidence levels

| Evidence | Establishes |
| --- | --- |
| Live | Exact recorded build/adapter/workflows. |
| Fixture | Mocked/filesystem/process/reducer/UI behavior. |
| Static/host | Build/lint/terminal checks without live BitBake. |
| Not validated / Blocked | Missing live validation / missing prerequisite. |

## Protocol and backend matrix

Bundled Tinfoil bridge is normal; process backend has fewer metadata capabilities.
Environment-only/custom bridges are diagnostic and inherit no live-control proof.
Daemon IPC requires exact handshake and instance/generation/sequence validation.
See [Protocols](bridge-and-daemon-protocols.md).

## Observed live Yocto combination

2026-08-19 records: [Wrynose 6.0.2 / BitBake 2.18.0](compatibility-evidence/latest.toml)
and [Scarthgap 5.0.19 / BitBake 2.8.1](compatibility-evidence/older.toml).
They cover identity/probes/Doctor, recipes/layers/configuration, utility capabilities,
native task/log flow, base-files:do_listtasks completion, and bounded cancellation.
Records expire and require renewal after relevant changes; they do not identify
today's newest release. The retained 2.19.0 development observation is partial.

## Workflow compatibility matrix

Core inventory/task/cancellation evidence does not certify every Devtool/SDK/QA/
Security/Maintenance mutation, image boot, device write, SSH, or board debugger.
Rootfs needs exact generated paths; debugging needs matching symbols/transport.
Revalidate the requested workflow/release rather than extending fixture coverage.

## Host, runtime, and hardening matrix

Linux plus the chosen Yocto release's host prerequisites; ordinary UI needs 80x24.
CI pins Rust 1.97.0/Python 3.12 and raster Cairo/PyCairo/DejaVu; exact screenshot
checks need those fonts. PDFs need graphics/document tools. Fuzz/sanitizer/resource
checks establish only their tested boundary.

## Adding a supported live combination

Choose exact official revisions, initialize an isolated build, record binary/
source/build/machine/distro/host identity, then run [live tests](testing.md#live-yocto-validation).
Core smoke does not complete an image. Maintained-role validation uses
YOCTUI_LIVE_COMPATIBILITY=1 with test-compatibility-matrix.sh --live latest/older;
development additionally needs an exact revision and cannot establish support.
Retain commands/outcomes/limits/hashes/freshness; offline --evidence-only validates
records without fetching/building. Missing prerequisites remain blockers.
