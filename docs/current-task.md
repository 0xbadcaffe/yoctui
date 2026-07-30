# Current task

## Active task

**ID:** QA-SPEC-001
**Title:** Specify unified QA workflows

## Objective

Define authoritative, capability-driven recipe, kernel, and configured-layer
QA behavior before implementation.

## Required work

1. Specify a first-class QA Navigator destination with responsive Recipe &
   Kernel and Layer QA views, exact selection, search, Inspector, and footer
   behavior.
2. Define authoritative capability discovery and check catalogs for kernel
   configuration, URI, patch, license, general recipe QA, and
   `yocto-check-layer` without guessing task or tool support.
3. Define exact recipe/kernel/layer scopes, deterministic indexed previews,
   managed BitBake reuse, independent layer execution, cancellation,
   navigation retention, and terminal outcomes.
4. Define bounded typed finding/report identities, acquisition, imports,
   filters, limitations, refresh, and exact editor/provider/report routes.
5. Assign pure state, process/filesystem parsing, key/effect mapping,
   rendering, and polling to the correct architecture layers.
6. Reconcile existing recipe task and patch-review routes without duplicating
   or weakening them.
7. Keep mocked evidence and live Yocto compatibility claims separate.

## Definition of done

- `docs/ui-spec.md` completely defines QA interaction and responsive behavior.
- `docs/architecture.md` defines typed ownership and dependency boundaries.
- Existing recipe QA and patch-review routes are reconciled with the new
  destination.
- The registry's dependent implementation tasks remain coherent.

## Verification

```bash
./scripts/verify-roadmap.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
