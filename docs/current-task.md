# Current Task

**ID:** M68-GALLERY-001
**Title:** Update front README gallery with all repaired workflows
**Status:** IN_PROGRESS

Dependencies: M68-PIE-001 (DONE).

Scope and done criteria: production fixtures, reviewed cell goldens, rasters, manifest, README and release version; all requested screens, baseline, docs and screenshot verification. Update UI/architecture where changed, registry, status and current task; baseline checks and one coherent commit required.

Verification: `python3 scripts/render-readme-screenshots.py --check`, `python3 scripts/render-m22-concept-screenshots.py --check` plus AGENTS.md baseline.
