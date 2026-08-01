# Current Task

## Task

**ID:** DOC-INSTALL-001
**Title:** Document installation and Yocto quickstart

## Objective

Make a fresh Yoctui checkout buildable and launchable with copyable commands
for both the current `bitbake-setup` workflow and an existing Poky environment,
then guide the first image build entirely from the TUI.

## Required work

1. Reconcile README commands with the current CLI help, bundled scripts, and
   the two distinct environment layouts; do not mix standalone BitBake and
   Poky paths.
2. Add explicit supported host/tool prerequisites, Rust installation check,
   repository clone/build/install commands, and binary locations.
3. Keep one copyable current-development `bitbake-setup` block and one copyable
   existing-Poky `oe-init-build-env` block. Validate setup scripts before
   sourcing them and explain the README-only Poky master migration case.
4. Launch Yoctui without a positional target so opening the workspace never
   starts a remembered or shell-requested build. Describe selecting the target
   and confirming the first `core-image-minimal` build inside the TUI.
5. Document bridge versus process backend selection, non-Yocto smoke/doctor
   commands, configuration isolation for CI, and where to find full operator,
   testing, profiling, and compatibility guidance.

## Definition of done

- A fresh repository checkout has complete build/install commands.
- Both supported environment paths are distinct, copyable, and guarded by
  file checks.
- The first real image build begins only from explicit TUI controls.
- CLI help, safe CLI smoke tests, links, and baseline verification pass.

## Verification

```bash
test -s README.md
cargo run -q -p yoctui -- --help
./scripts/test-cli.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `README.md` only; link detailed guidance rather than duplicating it.
- Mark `DOC-INSTALL-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `DOC-OPERATOR-001`.

## Next task

`DOC-OPERATOR-001`
