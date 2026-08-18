# Current Task

## Task

**ID:** COMPAT-ENV-ID-001
**Title:** Add typed Yocto environment identity
**Status:** IN_PROGRESS

## Objective

Add a pure typed model containing only authoritative detected identity for the
selected Yocto/OpenEmbedded/BitBake environment, with every unavailable value
represented explicitly as Unknown.

## Dependencies

- `COMPAT-SPEC-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Typed fields cover BitBake, OE-Core/Poky, distro, machine, layer-series,
  canonical build/source roots, environment tooling, and backend/protocol.
- Unknown is explicit per field; weak heuristics cannot construct authority.
- Identity normalization is bounded, deterministic, and tested for invalid,
  duplicate, mixed-layer, and partial environments.
- Serialization remains suitable for the later bounded protocol task without
  importing protocol or backend types into the model.
- Focused tests and required documentation/roadmap checks pass.

## Verification

```bash
cargo test -p yoctui-model compatibility::environment_identity
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
