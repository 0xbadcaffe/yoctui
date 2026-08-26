# Current Task

## Task

**ID:** UX-THROBBER-001
**Title:** Adopt one accessible indeterminate activity language
**Status:** NOT_STARTED

## Objective

Evaluate the admitted throbber candidate and provide one reducer-tick-owned,
accessible activity language without fabricating progress.

## Dependencies

- `UX-PROGRESS-001` — DONE
- `UX-LICENSE-001` — DONE

## Relevant files

- reducer-owned animation ticks and preferences
- shared activity rendering
- admitted dependency evidence and manifests if adopted
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Activity symbols are derived only from reducer-owned ticks.
- Reduced motion uses stable text.
- ASCII and no-color retain lifecycle meaning.
- Terminal states never retain an active throbber.
- Unknown activity never becomes fabricated numeric progress.
- Any adopted dependency passes the existing admission and deny gates.

## Verification

```bash
cargo test -p yoctui-model ux_throbber
cargo test -p yoctui-ui ux_throbber
cargo deny check
./scripts/verify-roadmap.sh
```
