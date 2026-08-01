# Current Task

## Task

**ID:** DOC-001
**Title:** Complete operator and compatibility documentation

## Objective

Close the documentation parent gate only after every atomic documentation task
and the combined installation, operator, compatibility, and deterministic
validation surface pass together.

## Required work

1. Verify `DOC-INSTALL-001`, `DOC-OPERATOR-001`, `DOC-COMPAT-001`, and
   `DOC-VERIFY-001` are `DONE` and their committed files are present.
2. Run the parent verification exactly; fix any disagreement rather than
   weakening the checker or duplicating guidance.
3. Run the baseline before closing this parent. After it is closed, run the
   repository completion gate and preserve any external hardening or
   live-validation blocker exactly as reported.
4. Mark `DOC-001` `DONE`, update the implementation status, and select the next
   eligible task. Do not claim overall completion while a required blocked task
   remains.

## Definition of done

- Every documentation child task is `DONE` with passing evidence.
- README, operator guide, compatibility matrix, and local documentation gate
  pass together.
- Documentation status is closed without changing live-support claims.
- Baseline verification passes and the strict completion result is recorded
  honestly.

## Verification

```bash
test -s docs/compatibility.md
test -s docs/operator-guide.md
test -s README.md
./scripts/check-docs.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Post-task completion audit

After the documentation task is committed, run:

```bash
./scripts/verify-completion.sh
```

This audit determines overall repository completion. An unrelated required
task or external host blocker does not retroactively invalidate documentation
that passed its registry verification, but it must remain explicit and prevents
the repository from being declared complete.

## Documentation updates

- Mark `DOC-001` `DONE` only after its own verification passes.
- Update `docs/implementation-status.md` and the milestone summary.
- Replace this file with the next eligible task or the exact remaining blocked
  gate reproduction.

## Next task

`HARDEN-ANALYSIS-001` after its host perf permission is available; otherwise
record that it remains the external completion blocker.
