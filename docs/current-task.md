# Current Task

## Task

**ID:** UX-IMAGE-PREVIEW-001
**Title:** Evaluate optional terminal image previews
**Status:** NOT_STARTED

## Objective

Determine whether terminal-native image previews improve Yoctui without
weakening portability, bounded rendering, startup size, or deterministic text
fallbacks, then either admit a narrowly configured renderer or record a tested
rejection and retain the current artifact presentation.

## Dependencies

- `UX-WIDGET-PRIMITIVES-001` — DONE
- `UX-LICENSE-001` — DONE

## Relevant files

- terminal graphics capability/protocol probing and SSH/tmux behavior
- bounded asynchronous image decode and resize ownership
- dependency features, license evidence, MSRV, binary size, and memory budgets
- deterministic half-block and text fallbacks for TestBackend/accessibility
- Images preview lifecycle, cancellation, resize, and production UI tests

## Definition of done

- Terminal protocol support is explicit and never inferred from image content;
  SSH, tmux, unsupported terminals, and tests retain useful deterministic text.
- Decode/resize is cancellable, off the draw path, and bounded by pixel, byte,
  time, generation, and retained-memory limits.
- Native/default features and binary-size impact are measured before admission;
  license, notice, SBOM, locked/offline, and deny gates remain current.
- Focused UI/CLI tests prove every supported, fallback, stale, failure, resize,
  and cancellation path; a rejection is acceptable only with recorded evidence.

## Verification

```bash
cargo test -p yoctui-ui ux_image_preview
cargo test -p yoctui -- ux_image_preview
cargo deny check
```
