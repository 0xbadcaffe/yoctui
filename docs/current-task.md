# Current Task

## Task

**ID:** ROOTFS-PKGDATA-VIEWPORT-001
**Title:** Stream large rootfs pkgdata and retain visible selection
**Status:** DONE

## Objective

Rootfs composition must accept valid current-release generated pkgdata without
losing its safety bounds, and every bounded collection must keep its selected
row visible when navigation reaches the end.

## Dependencies

- UX-EXTERNAL-REDRAW-001 — DONE

## Definition of done

- Runtime pkgdata is parsed incrementally with explicit per-file, per-line, and
  aggregate byte bounds.
- Current scoped `PKGSIZE:<package>` and `FILES_INFO:<package>` records retain
  exact installed-size and file-count evidence.
- One over-bound package degrades installed-package authority to Partial rather
  than failing the entire Rootfs composition screen.
- Every workspace table, list, and bounded picker centers a viewport around its
  selected identity, including the final row.
- Version 0.1.9 is installed and repository completion gates pass.

## Verification

```bash
cargo test -p yoctui-bitbake ux_rootfs
cargo test -p yoctui-bitbake --test live_rootfs -- --ignored
cargo test -p yoctui-ui ux_scrollable_collection_matrix_keeps_the_last_highlighted_row_visible
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

The reported Rootfs failure came from a legitimate 1.1 MiB `kernel-devsrc`
runtime-pkgdata record exceeding a legacy 256 KiB whole-file limit. The adapter
now streams bounded lines, counts the large JSON object without materializing
it, and preserves partial authority at a single-package limit. The scroll audit
also removed full-list rendering from independently clipped workspaces and
pickers, so the render viewport follows the model-owned selection.
