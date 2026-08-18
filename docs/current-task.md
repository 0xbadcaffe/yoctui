# Current Task

## Task

**ID:** COMPAT-DAEMON-RUNTIME-001
**Title:** Install probed compatibility authority in the production daemon
**Status:** IN_PROGRESS

## Objective

Make the production daemon actually identify, probe, own, and publish the
capability snapshot for the initialized Yocto build environment in which it
starts.

## Dependencies

- `COMPAT-ENV-ID-001` — DONE
- `COMPAT-PROBE-001` — DONE
- `COMPAT-DAEMON-001` — DONE
- `COMPAT-DOCTOR-001` — DONE
- `COMPAT-BITBAKE-GETVAR-001` — DONE

## Relevant files

- `crates/yoctui-cli/src/daemon_compatibility.rs`
- `crates/yoctui-cli/src/main.rs`
- `crates/yoctui-model/src/daemon_state.rs`
- `crates/yoctui-app/src/daemon_state.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Daemon startup in an initialized environment derives one authoritative,
  bounded identity/context from `BUILDDIR`, initialized tools, exact BitBake
  version output, build configuration, layer configuration, and bridge/backend
  protocol evidence without weak release-name heuristics.
- The coordinator probes once and installs the normalized snapshot into daemon
  model state, the initial journal snapshot, Doctor transport, and every
  capability-aware command supervisor before clients attach.
- Startup outside an initialized Yocto build remains safe and explicitly has
  no compatibility authority; host PATH alone never creates one.
- Runtime tests use fake initialized tools/configuration to prove exact
  identity, probing, publication, missing-environment behavior, and bounded
  failure handling.

## Verification

```bash
cargo test -p yoctui --bin yoctui daemon_compatibility_runtime
cargo test -p yoctui --bin yoctui doctor_compatibility
./scripts/verify-roadmap.sh
```
