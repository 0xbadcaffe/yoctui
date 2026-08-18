# Current Task

## Task

**ID:** COMPAT-UNKNOWN-001
**Title:** Handle future Yocto releases conservatively
**Status:** IN_PROGRESS

## Objective

Ensure an unknown future Yocto/Poky/OE-Core/BitBake release is never rejected
by its unfamiliar name or version, while enabling only behavior backed by
positive current-environment evidence.

## Dependencies

- `COMPAT-VERSION-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility.rs`
- `crates/yoctui-bitbake/src/compatibility_version.rs`
- `docs/compatibility.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Future/unknown release identity remains valid and inspectable.
- Positive direct probes enable their exact capability despite unknown release.
- Inconclusive/absent capabilities remain Unknown and disabled.
- Static historical fallbacks never cross their documented upper boundary.
- Synthetic future tests cover mixed positive, negative, absent, conflict, and
  fallback evidence without rejecting the application/environment.

## Verification

```bash
cargo test -p yoctui-bitbake compatibility_future_unknown
cargo test -p yoctui-model compatibility_future_unknown
./scripts/verify-roadmap.sh
```
