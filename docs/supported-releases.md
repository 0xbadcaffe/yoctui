# Supported releases

Exact observations, not blanket BSP/point-version support. See [Compatibility](compatibility.md).

## Classification vocabulary

| Classification | Meaning |
| --- | --- |
| Claimed supported | Current live records satisfy required policy/workflows. |
| Tested | Exact revision/workflows observed; broader support not implied. |
| Partially tested | Live observations do not cover the full support set. |
| Expected compatible | Probe/catalog prediction, no live support claim. |
| Unsupported | Policy/evidence excludes the required baseline. |
| Unknown | Missing, stale, conflicting, or insufficient evidence. |

## Current matrix

| Yocto/Poky identity | BitBake | Classification | Evidence |
| --- | --- | --- | --- |
| Wrynose 6.0.2, official split composition, 2026-08-19 | `2.18.0` | Claimed supported | [Latest compatibility record](compatibility-evidence/latest.toml): identity/inventories/core task/cancellation; exact source revisions. |
| Scarthgap LTS 5.0.19, official Poky, 2026-08-19 | `2.8.1` | Claimed supported | [Older compatibility record](compatibility-evidence/older.toml): same core boundary with degraded capabilities; exact source revision. |
| 6.0.99+snapshot-a4eb7bc2a750f76d9772eb88b7afb2b801bd1250, 2026-07-24 | `2.19.0` | Partially tested | [Focused compatibility observation](compatibility.md#observed-live-yocto-combination); not release support. |
| Other releases | Unknown | Unknown | No exact compatibility record. |

## Support window

Recorded lower anchor: Scarthgap 5.0.19 / 2.8.1; upper: Wrynose 6.0.2 / 2.18.0.
Other point revisions need evidence. Future/development and mixed identities: **Unknown**;
individual positively detected capabilities may still work.

## Evidence policy

latest.toml and older.toml record exact official sources/commits, observation/expiry,
Yoctui binary/source, initialized build/distro/machine, backend/protocol, commands,
capabilities, and workflows. They must be live and fixture_only=false. Renew after
expiry or relevant contract changes. Official selections are refreshed for each run.
Fixtures (legacy 1.46, modern 2.8/2.18, development 2.19, future 99.0) test resolver
boundaries and probe precedence, not official release selection or support.

## Validation

`./scripts/test-compatibility-matrix.sh --evidence-only` checks identity, expiry,
source composition, ordering, and policy without fetching/building.
YOCTUI_LIVE_COMPATIBILITY=1 enables --live latest/older: private source/build/runtime,
Doctor/inventory, bounded native task/cancellation, artifacts/compatibility/<role>.
It never rewrites tracked records. --live development needs
YOCTUI_COMPAT_DEVELOPMENT_REVISION and remains diagnostic.
