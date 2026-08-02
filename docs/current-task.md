# Current Task

## Task

**ID:** DOC-README-001
**Title:** Refresh README with visual project overview
**Status:** IN_PROGRESS

## Objective

Make the repository landing page concise, visual, and immediately useful
without weakening its guarded Yocto setup instructions or evidence labels.

## Scope

- Replace the long, repetitive README flow with a compact project overview.
- Keep one copyable guarded installation and Yocto launch path.
- Embed `docs/media/yoctui-demo.gif`, captured from the real binary with
  clearly labelled deterministic fixture metadata.
- Embed/link the real perf-backed `artifacts/flamegraph/yoctui.svg`.
- Preserve links to detailed operator, compatibility, testing, profiling, UI,
  and architecture documentation.

## Verification

```bash
test -s README.md
test -s docs/media/yoctui-demo.gif
test -s artifacts/flamegraph/yoctui.svg
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```

## Definition of done

- The README is materially shorter and has no duplicated run instructions.
- Its quickstart commands are directly copyable and retain failure guards.
- The GIF renders the actual Yoctui binary and is labelled as fixture-backed.
- The Flamegraph is visible and described as real perf evidence.
- Documentation and roadmap verification pass.

## Next task

Return to the terminal completed state after this task passes.
