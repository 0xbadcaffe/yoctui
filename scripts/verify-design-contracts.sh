#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
for document in docs/development.md docs/interface-behavior.md docs/architecture.md \
  docs/widgets-and-dependencies.md docs/design/README.md docs/design/ui-scenes.toml; do
  test -s "$document" || {
    printf 'missing or empty design contract: %s\n' "$document" >&2
    exit 1
  }
done
./scripts/verify-workbench-design.sh
python3 scripts/verify-m21-concept-screens.py --fixtures-only
