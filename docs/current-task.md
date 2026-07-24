# Current task

## Active task

**ID:** LAYERS-001
**Title:** Complete lazy Layers tree and contextual Inspector

## Objective

Complete the IDE-style configured-layer tree and file/directory Inspector
specified by `docs/ui-spec.md` without eagerly scanning the Yocto source tree.

## Required work

1. Inventory the existing configured-layer list, one-directory browser,
   preview loader, editor effects, relationships, metadata search, and tests.
2. Represent a lazily loaded expandable/collapsible tree with stable path
   identity and bounded selection; never recursively scan unopened subtrees.
3. Keep every configured layer visible with priority, compatibility, and
   active-build highlighting.
4. Sort directories before files and implement hidden-file toggling without
   losing the current path or selection.
5. Filter configured layers, loaded paths, and filenames through the existing
   search workflow.
6. Detect modified, untracked, ignored/generated, and clean state with Git
   where available; make missing Git visible rather than fatal.
7. Populate the contextual Inspector for directories and files with full path,
   size, modification/Git state, relationships, and safe preview metadata.
8. Add line-numbered, syntax-aware text previews; identify binary content and
   truncate/stream large files at a documented bound.
9. Implement the specified expand/collapse/open, editor, refresh, hidden, Git,
   metadata, and dependency input routes as typed actions/effects.
10. Cover empty layers, deep lazy navigation, refresh, hidden files, Git,
    binary/large previews, external-open failures, and all responsive modes
    with reducer, integration, input, and `TestBackend` tests named
    `layer_tree`.

## Definition of done

- All configured layers remain visible and only expanded paths are scanned.
- Tree expansion/collapse, hidden toggling, filtering, refresh, and selection
  are bounded and deterministic.
- Active, compatibility, priority, relationships, and Git state are visible.
- File/directory Inspector content is safe for text, binary, and large files.
- Every specified keyboard operation maps through typed actions/effects.
- Responsive rendering and external-tool failures never panic.
- Reducer, integration, input-mapping, and TestBackend checks pass.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model layer_tree
cargo test -p yoctui-ui layer_tree
cargo test -p yoctui-app layer_tree
cargo test -p yoctui -- layer_tree
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`RECIPES-001 — Complete recipes workspace actions`
