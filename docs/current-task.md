# Current Task

## Task

**ID:** SHELL-DOC-001
**Title:** Document embedded shell behavior and safety
**Status:** NOT_STARTED

## Objective

Document input ownership, escape chord, supported terminal behavior,
environment/cwd choices, session limits, paste/OSC policy, child cleanup, and
troubleshooting.

## Verification

```bash
test -s docs/embedded-shell.md
./scripts/check-docs.sh
```

## Definition of done

- Embedded shell behavior and safety policy match the shipped model/backend.

## Next task

After completion, select `SHELL-001`.
