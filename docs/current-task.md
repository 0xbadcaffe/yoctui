# Current Task

## Task

**ID:** COMPAT-VERSION-001
**Title:** Add release and version fallback mapping
**Status:** IN_PROGRESS

## Objective

Add one documented, testable fallback map for the capabilities that cannot be
probed directly, without allowing release comparisons to leak into UI,
workspace, or command code.

## Dependencies

- `COMPAT-PROBE-001` — DONE

## Relevant files

- `crates/yoctui-bitbake/src/compatibility_version.rs`
- `crates/yoctui-bitbake/src/lib.rs`
- `crates/yoctui-model/src/compatibility_catalog.rs`
- `docs/compatibility.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Versions are parsed and compared centrally with explicit component identity.
- Fallback rules map only catalog-declared unprobeable behavior to typed state,
  implementation, reason, and fallback evidence.
- Direct positive/negative evidence has precedence over fallback inference.
- Unknown/malformed/future versions default conservatively and never inherit
  historical behavior implicitly.
- Rules and authoritative sources are documented and covered by boundary,
  precedence, malformed, and future-version tests.

## Verification

```bash
cargo test -p yoctui-bitbake compatibility_version
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
