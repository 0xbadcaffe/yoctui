# Current Task

## Task

**ID:** CRATESIO-COVERAGE-001
**Title:** Realign Python quality checks with the bundled bridge
**Status:** DONE

## Objective

Point Python lint, formatting, type, and coverage checks at the canonical
bridge source bundled in `yoctui-bitbake`, while retaining the external bridge
tests, so the terminal completion gate measures the shipped implementation.

This is the terminal handoff: every registered task is `DONE`.

## Dependencies

- `CRATESIO-PUBLISH-001` — DONE

## Relevant files

- `pyproject.toml`
- `scripts/verify-completion.sh`
- `crates/yoctui-bitbake/bridge/yoctui_bridge.py`
- `bridge/tests/`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Ruff checks the canonical bridge and its external tests.
- Ruff formatting checks the canonical bridge and its external tests.
- Mypy checks the canonical bridge and its external tests.
- Pytest coverage measures the canonical bridge and clears 75%.
- The terminal completion gate passes.

## Completion evidence

- Ruff, formatting, and mypy pass for packaged source and external tests.
- All 39 bridge tests pass.
- Packaged-source coverage is 75.95% against a 75% minimum.

## Verification

```bash
$HOME/.local/bin/ruff check crates/yoctui-bitbake/bridge bridge/tests
$HOME/.local/bin/ruff format --check crates/yoctui-bitbake/bridge bridge/tests
$HOME/.local/bin/mypy crates/yoctui-bitbake/bridge bridge/tests
$HOME/.local/bin/pytest bridge/tests --cov=crates/yoctui-bitbake/bridge --cov-report=term-missing --cov-fail-under=75
./scripts/verify-completion.sh
```
