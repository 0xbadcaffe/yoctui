# Current Task

## Task

**ID:** CRATESIO-PUBLISH-001
**Title:** Publish and validate yoctui 0.1.0 on crates.io
**Status:** IN_PROGRESS

## Objective

Publish the verified `yoctui` 0.1.0 package graph to crates.io, validate an
installation from immutable registry artifacts, and retain exact release
commit and tag evidence.

## Dependencies

- `CRATESIO-PACKAGE-001` — DONE

## Relevant files

- crates.io package records
- Git release commit and `v0.1.0` tag
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Required support crates are published in dependency order.
- `yoctui` 0.1.0 is published after dependency propagation.
- A clean `cargo install yoctui --version 0.1.0 --locked` succeeds from crates.io.
- The installed binary's version, help, and embedded bridge smoke pass.
- The exact release commit is tagged `v0.1.0` and release evidence is recorded.

## Verification

```bash
cargo install yoctui --version 0.1.0 --locked
yoctui --version
yoctui --help
./scripts/verify-roadmap.sh
```
