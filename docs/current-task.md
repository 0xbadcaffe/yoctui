# Current Task

## Task

**ID:** IMAGE-UDEV-001
**Title:** List image-owned udev rules with bounded offline inspection
**Status:** IN_PROGRESS

## Objective

Add a sixth Images/rootfs tab for the exact reported IMAGE_ROOTFS udev rules,
including logical paths, precedence/masking evidence, and bounded preview.
Never inspect host rules, execute rules, or infer live device state.

## Dependencies

- WORKBENCH-INTEGRATION-001 — DONE

## Definition of done

- Vendor, runtime, and administrative rules are visible, including shadowed
  and masked entries, with explicit partial/unavailable states.
- Selection, preview, scrolling, safety, and narrow rendering have tests.
- UI specification, architecture, registry, and status agree.

## Verification

```bash
cargo test -p yoctui-model udev
cargo test -p yoctui-bitbake udev
cargo test -p yoctui-ui udev
cargo test -p yoctui-app udev
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```

Next: CONSOLE-TERM-002, YOCTO-LOGGER-ADAPTER-001, LOG-CONSOLE-IMAGE-001.
The independent completion gate remains mandatory at the final boundary.
