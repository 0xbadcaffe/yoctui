# Current Task

## Task

**ID:** LIVE-UI-POKY-001
**Title:** Validate redesigned UI against real Poky
**Status:** IN_PROGRESS

## Objective

Run the redesigned workbench against a fresh supported Poky environment and
retain policy-complete real-binary terminal evidence across startup, metadata,
build, logs, completion, safe failure, terminal interaction, and reconnect.

## Dependencies

- `VISUAL-TEST-003` — DONE
- `PTY-UI-TEST-001` — DONE
- `PERF-UI-002` — DONE

## Relevant files

- `scripts/test-live-next-generation-ui.sh`
- `scripts/verify-next-generation-ui-evidence.sh`
- `artifacts/release-quality/next-generation-ui/`
- `docs/ui-acceptance-tests.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The harness uses a fresh supported Poky environment and the real release
  Yoctui binary, with exact revision, BitBake version, host, MACHINE, DISTRO,
  build directory, commands, binary identity, and outcomes recorded.
- Startup, environment verification, Recipes, Layers, Tasks,
  `core-image-minimal`, live logs, successful completion, a safe failure,
  terminal interaction, and daemon attach/reconnect are exercised.
- Raw ANSI, semantic terminal text, terminal dimensions, process/build logs,
  exit states, and representative terminal buffers are retained and bounded.
- The independent verifier rejects fixture evidence, missing/redacted policy
  fields, stale binary/commit identities, incomplete scenarios, and oversize
  artifacts.

## Verification

```bash
./scripts/test-live-next-generation-ui.sh
./scripts/verify-next-generation-ui-evidence.sh
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
