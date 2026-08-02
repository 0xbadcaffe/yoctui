# Current Task

## Task

**ID:** DOC-README-001
**Title:** Refresh README with visual project overview
**Status:** DONE

## Objective

Make the repository landing page concise, visual, and immediately useful
without weakening its guarded Yocto setup instructions or evidence labels.

## Completed evidence

- README shrank from 1,811 to 675 words while retaining guarded installation
  and current-development/existing-Poky launch paths.
- `docs/media/yoctui-demo.gif` is a 952x484 capture of the real binary cycling
  through Dashboard, layer-scoped recipes, Recipes, and Help with explicit
  deterministic-fixture labelling.
- The perf-backed `artifacts/flamegraph/yoctui.svg` is embedded and linked.
- `scripts/check-docs.sh` requires both visual artifacts to remain nonempty.
- Detailed operator, compatibility, testing, profiling, UI, architecture, and
  implementation-evidence links remain available.

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

None. All 155 registry tasks are complete.
