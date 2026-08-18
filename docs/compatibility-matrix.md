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
| Poky `6.0.99+snapshot-a4eb7bc2a750f76d9772eb88b7afb2b801bd1250` observed 2026-07-24 | `2.19.0` | Partially tested | Exact [live bridge/Tinfoil observation](compatibility.md#observed-live-yocto-combination): core smoke and selected focused workflows only; predates the M18 release-support gate and is not a support claim. |
| Any other release | Unknown | Unknown | No M18 live compatibility evidence yet. |

## Support window

- Minimum supported release: **not claimed**; requires current
  `COMPAT-LIVE-OLDER-001` non-fixture evidence.
- Latest supported stable release: **not claimed**; requires current
  `COMPAT-LIVE-LATEST-001` non-fixture evidence selected from authoritative
  Yocto release documentation at validation time.
- Latest exact live observation: the partially tested development snapshot in
  the table above; it is neither a minimum nor a latest-stable claim.
- Future/development and mixed identities: **Unknown** at release-policy level;
  positively detected individual capabilities may still run.

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
authoritative Yocto Project documentation when the live task is executed; this
file deliberately does not guess future release names during milestone setup.
