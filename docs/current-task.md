# Current Task

## Task

**ID:** RELVAL-CI-002
**Title:** Validate GitHub Actions workflow syntax
**Status:** DONE

## Objective

Final completed task: GitHub Actions workflow syntax validation; release-quality
validation, utility workbench, and embedded native shell requirements are complete.

## Verification

```bash
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml")'
git diff --check
```

## Definition of done

- The workflow parses as valid YAML and the diff is whitespace-clean.

## Next task

## Terminal handoff

All registry tasks are complete. The aggregate completion gate is blocked on
the host's disabled Linux perf sampling (`perf_event_paranoid=4`); all checks
before the perf-backed flamegraph pass.
