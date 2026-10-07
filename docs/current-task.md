# Current Task

**ID:** DEMO-INSTALL-LIVE-001
**Title:** Deferred optimized installation and live demo handoff
**Status:** IN_PROGRESS

The broader demo queue remains deferred until the user explicitly resumes it.
The current request is documentation cleanup on `feat_prod` only. Do not resume
unrelated demo, reboot, publication, daemon, or image-build work.

PROD-HOST-PRIVACY-001 is DONE at version 0.1.316. See
[host privacy](testing/host-privacy.md) and the retained receipt at
`artifacts/release-quality/host-privacy-0.1.316.json`.
Published crates.io release 0.1.315 is recorded separately at
`artifacts/release-quality/cratesio/0.1.315.json`.

Task dependencies, acceptance criteria, and verification commands are retained
in `docs/task-registry.toml`. Completed implementation history is maintained in
`docs/implementation-status.md` and `docs/product-roadmap.md`; historical live
observations remain in the testing and performance documents and artifacts.

## Documentation cleanup verification

```bash
./scripts/verify-roadmap.sh
./scripts/test-readme-quickstart.sh
python3 scripts/check-library-layout.py
python3 scripts/check-version-bump.py
cargo fmt --all --check
git diff --check
```

## Deferred live verification

The existing task still requires a source-bound optimized installation, genuine
native OpenBMC screen/session rehearsal, exact daemon/workspace authority, and
owned-process cleanup. See its registry entry for the complete manual checks.
Documentation cleanup does not establish live acceptance or authorize stopping
unrelated sessions, building images, publishing releases, or rebooting.
