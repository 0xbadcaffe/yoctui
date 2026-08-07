# Current Task

## Task

**ID:** RELVAL-HARDEN-001
**Title:** Make Security mapper launches resilient to transient ETXTBSY
**Status:** DONE

## Objective

Final completed task: eliminate the transient `Text file busy` process-launch
failure while preserving bounded retries and Security mapper semantics.

## Verification

```bash
cargo test -p yoctui-bitbake security_mapper
./scripts/verify-completion.sh
```

## Definition of done

- Security mapper retries only transient `ETXTBSY` errors with a bounded delay.
- The full completion gate passes.

## Next task

## Terminal handoff

## Terminal handoff

All registry tasks are complete; run the aggregate completion gate from this
committed checkout.
