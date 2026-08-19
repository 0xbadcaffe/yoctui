# Yocto Release Capability Matrix

This matrix is the release-policy index for Yoctui's environment-correlated
functionality. Existing observations remain recorded in
[compatibility.md](compatibility.md). M18 does not promote fixture coverage or
an isolated successful operation into a release support claim.

## Classification vocabulary

| Classification | Meaning |
|---|---|
| Claimed supported | Current live evidence satisfies the repository policy for the stated release and required workflow set. |
| Tested | Current live evidence exists for the exact recorded revision and listed workflows; broader release support is not implied. |
| Partially tested | Current live evidence exists but does not cover the required support workflow set. |
| Expected compatible | Catalog/probe evidence suggests compatibility, but the repository makes no live support claim. |
| Unsupported | Policy or authoritative evidence establishes that Yoctui cannot safely provide the required baseline. |
| Unknown | Evidence is absent, stale, contradictory, or insufficient. |

## Current matrix

| Yocto/Poky identity | BitBake | Classification | Evidence |
|---|---|---|---|
| Official Poky component composition `6.0.2` (Wrynose), exact revisions observed 2026-08-19 | `2.18.0` | Tested | Current non-fixture [latest compatibility evidence](compatibility-evidence/latest.toml): daemon identity/probes, Doctor, 1,922 Recipes, 3 Layers, Configuration, Devtool/utilities, native task/log events, exit-0 core task, and bounded cancellation. Exact revision scope only; the full support-window claim awaits older-release evidence. |
| Poky `6.0.99+snapshot-a4eb7bc2a750f76d9772eb88b7afb2b801bd1250` observed 2026-07-24 | `2.19.0` | Partially tested | Exact [live bridge/Tinfoil observation](compatibility.md#observed-live-yocto-combination): core smoke and selected focused workflows only; predates the M18 release-support gate and is not a support claim. |
| Any other release | Unknown | Unknown | No M18 live compatibility evidence yet. |

## Support window

- Minimum supported release: **not claimed**; requires current
  `COMPAT-LIVE-OLDER-001` non-fixture evidence.
- Latest stable exact revision tested: **Yocto 6.0.2 (Wrynose), BitBake
  2.18.0**. It was selected from the authoritative release calendar and 6.0.2
  release notes. A broader supported-window claim remains pending the required
  older-release baseline and final parent gate.
- Latest exact live observation: the partially tested development snapshot in
  the table above; it is neither a minimum nor a latest-stable claim.
- Future/development and mixed identities: **Unknown** at release-policy level;
  positively detected individual capabilities may still run.

## Deterministic fixture roles

The test-only fixture catalog has five policy roles: oldest-policy candidate,
intermediate representative, current-stable candidate, latest-support
candidate, and future/unknown. Candidate names are deliberately not official
release selections. Every record carries `fixture_only = true` and
`evidence_level = deterministic_fixture_only`; current-stable and latest
official identities remain Unknown until the live tasks select them from
authoritative Yocto documentation.

Fixtures exercise the BitBake `1.46` legacy fallback boundary, an intermediate
`2.8` modern fallback, the documented `2.18` upper mapped generation, the exact
partially tested `2.19.0` development observation, and synthetic `99.0.0`
future behavior. These values test resolver boundaries and direct-probe
precedence only. They do not add a matrix classification or release claim.

## Evidence policy

The final compatibility gate requires two machine-readable live records under
`docs/compatibility-evidence/`: `latest.toml` and `older.toml`. Each record must
identify the evidence schema, observation date, expiry policy, official source
URL, exact Poky repository and commit, Yocto release/series, BitBake version,
Yoctui commit, build directory identity, distro, machine, backend/protocol,
commands, and observed capability/workflow results. `evidence_level` must be
`"live"` and `fixture_only` must be `false`.

Recorded evidence expires after the policy interval encoded in the record and
must be renewed after a capability-contract change that affects the validated
workflow. Fixtures, fake processes, mocked Tinfoil, static version tables, and
successful parsing cannot satisfy either live record.

Official release selections for the live matrix must be refreshed from
authoritative Yocto Project documentation when the live task is executed. On
2026-08-19 the release calendar identified 6.0.2 as the newest published stable
and listed 6.0.3 for the following week, so an unreleased point version was not
selected.
