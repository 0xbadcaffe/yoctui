# Current Task

## Task

**ID:** DOC-VERIFY-001
**Title:** Validate documentation links and commands

## Objective

Add a deterministic local documentation gate that catches broken repository
links, missing operator coverage, stale CLI examples, unsafe smoke behavior,
and invalid checked-in shell scripts without requiring network or live Yocto.

## Required work

1. Inspect existing verification scripts and every repository Markdown link
   pattern before implementing; reuse existing safe CLI/headless helpers.
2. Add `scripts/check-docs.sh` with deterministic local checks for relative
   Markdown file links and anchors used by repository documentation. Reject
   missing files, directories used as documents, and invalid fragments without
   accessing the network.
3. Require the installation, operator, compatibility, testing, profiling,
   architecture, protocol, and UI documents plus the operator guide's required
   daily workflow and troubleshooting sections.
4. Verify current CLI help, the isolated no-Yocto headless workload, and doctor
   behavior without reading a developer session or starting BitBake.
5. Run `bash -n` over every checked-in `.sh` file using a deterministic sorted
   list, with actionable file-specific failures.
6. Integrate the documentation gate into CI and the completion gate without
   weakening any existing check. Document it in the testing guide.

## Definition of done

- Broken local Markdown links/fragments and required-section omissions fail
  with actionable output.
- CLI/help/headless/doctor checks are isolated and cannot start a remembered
  build.
- Every checked-in shell script receives syntax validation.
- CI and completion invoke the same gate.
- Focused and baseline verification pass.

## Verification

```bash
./scripts/check-docs.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Add `scripts/check-docs.sh` and update `docs/testing.md`.
- Update `.github/workflows/ci.yml` and `scripts/verify-completion.sh`.
- Mark `DOC-VERIFY-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `DOC-001`.

## Next task

`DOC-001`
