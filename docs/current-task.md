# Current Task

## Task

**ID:** RAW-EXEC-MODEL-001
**Title:** Add typed Raw execution lifecycle
**Status:** IN_PROGRESS

## Objective

Define the pure model lifecycle and bounded versioned protocol messages that
carry confirmed Raw work between clients and the daemon without granting
command-string or process authority.

## Dependencies

- `RAW-MODEL-001` — DONE
- `RAW-PREVIEW-001` — DONE
- `JOB-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Stable bounded Raw request, job, session, stream, and durable-reference
  identities are disjoint from generic build and PTY identities and reject
  empty, oversized, malformed, or cross-kind values.
- A confirmed execution request carries catalog revision, stable command ID,
  typed parameters, bounded additional argv, interaction/safety classes,
  reviewed capability generation, exact build-directory identity, and a
  deterministic digest of the indexed preview; it never carries a joined
  command string or executable authority.
- Pure lifecycle state distinguishes queued/starting/running/cancelling and
  terminal success/failure/cancelled/lost outcomes, elapsed timing, detach/
  attach state, cancellation intent, exit/result details, and durable job or
  PTY ownership without retaining transient PID or writer authority.
- Stdout and stderr remain typed, ordered, independently bounded streams with
  explicit retained-byte/line limits and dropped/truncated counts; replacement,
  duplicate, stale, and out-of-order events fail closed or remain reducer-inert.
- Versioned protocol DTOs and conversions enforce independent count/byte
  bounds, reject unknown required enum/identity variants, and round-trip every
  valid request, lifecycle event, bounded output chunk, snapshot, and result.
- App mappings translate protocol Raw messages mechanically into exact model
  actions/state without parsing output, joining argv, or constructing process
  commands, and preserve daemon sequence/generation correlation.
- Model, protocol, and app tests cover normal lifecycle transitions, every
  terminal and cancellation/detach path, bounds/overflow, malformed/stale/
  duplicate input, reconnect snapshots, Unicode byte limits, and fail-closed
  conversion behavior.

## Verification

```bash
cargo test -p yoctui-model raw_execution
cargo test -p yoctui-protocol raw_execution
cargo test -p yoctui-app raw_execution
cargo clippy -p yoctui-model -p yoctui-app -p yoctui-ui --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
