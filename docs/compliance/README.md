# Dependency Compliance Artifacts

Yoctui separates dependencies shipped by the workspace from widget candidates
that have only been evaluated.

- `THIRD_PARTY_NOTICES.md` is generated from the exact workspace `Cargo.lock`.
  It lists every non-workspace package and embeds all packaged root-level
  license, notice, copying, and copyright material with byte-authoritative
  SHA-256 hashes.
- `yoctui.cdx.json` is the matching CycloneDX 1.5 component and dependency
  graph for the shipped lockfile.
- `widget-candidates.toml` records the exact crate version, registry checksum,
  source, repository, SPDX expression, declared MSRV, selected features,
  Ratatui 0.30 compatibility, transitive closure, owner task, and explicit
  adopt/defer/reject decision for every evaluated showcase widget.
- `widget-candidates.cdx.json` is the CycloneDX 1.5 graph resolved for those
  exact candidate versions and feature sets. It is audit evidence, not a list
  of shipped dependencies.

No candidate is a Yoctui dependency while its `admitted` field is false. The
implementing task must refresh the candidate record and candidate SBOM, set an
approved candidate to `admitted = true`, add the exact dependency and features,
regenerate shipped notices/SBOM, and pass both verification scripts plus
`cargo deny check`. Rejected candidates are not copied or adapted from showcase
code or assets.

Regenerate the shipped artifacts after an intentional lockfile change:

```bash
python3 scripts/third_party_compliance.py shipped --write
```

Refresh the candidate SBOM only from a disposable audit manifest containing
the exact candidate pins and selected features:

```bash
python3 scripts/third_party_compliance.py write-candidate-sbom \
  --manifest /path/to/audit/Cargo.toml
```

Verify the committed evidence and the locked offline build:

```bash
./scripts/verify-third-party-notices.sh
./scripts/verify-widget-dependencies.sh
cargo deny check
```
