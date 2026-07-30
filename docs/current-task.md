# Current task

## Active task

**ID:** SEC-SPEC-001
**Title:** Specify typed CVE and SPDX workflows

## Objective

Define authoritative, capability-driven CVE mapping/check and SPDX/SBOM
generation and report-viewing behavior before implementation.

## Required work

1. Specify a first-class Security Navigator destination with responsive CVE
   and SPDX views, exact selection, search, Inspector, and footer behavior.
2. Define authoritative capability discovery for CVE checks/package mapping
   and release-dependent SPDX/SBOM task names without guessing support.
3. Define exact recipe/image operation previews, confirmations, managed
   BitBake reuse, cancellation, navigation retention, and terminal outcomes.
4. Define bounded typed report identities, acquisition, CVE findings/package
   mappings, SPDX documents, metadata, limitations, refresh, and editor routes.
5. Assign pure state, process/filesystem parsing, key/effect mapping,
   rendering, and polling to the correct architecture layers.
6. Keep mocked evidence and live Yocto compatibility claims separate.

## Definition of done

- `docs/ui-spec.md` completely defines Security interaction and responsive
  behavior.
- `docs/architecture.md` defines typed ownership and dependency boundaries.
- The registry contains coherent dependent implementation tasks.
- Existing recipe shortcuts are reconciled without claiming report support
  that does not exist.

## Verification

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
