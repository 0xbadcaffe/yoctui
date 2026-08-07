# Current Task

## Task

**ID:** SHELL-001
**Title:** Complete the embedded native shell workspace
**Status:** NOT_STARTED

## Objective

Run the embedded-shell parent gate across session model, PTY backend, terminal
emulation, UI foundation, integration, PTY tests, and documentation.

## Verification

```bash
./scripts/test-embedded-shell.sh
```

## Definition of done

- Embedded shell sessions remain inside Yoctui with PTY lifecycle, bounded
  terminal state, safety policy, and restoration evidence.

## Next task

After completion, select `RELVAL-001`.
