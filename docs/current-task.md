# Current Task

## Task

**ID:** THEME-PACKRAT-003
**Title:** Preserve legacy theme configuration compatibility
**Status:** DONE

## Objective

Final completed task: preserve existing `dark` and `light` configuration values
while serializing the Packrat theme names.

## Verification

```bash
cargo test -p yoctui settings_session_overrides_config_but_cli_no_color_remains_authoritative
```

## Definition of done

- Legacy configuration values deserialize to their Packrat equivalents.

## Next task

## Terminal handoff

All registry tasks are complete.
