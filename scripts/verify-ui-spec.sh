#!/usr/bin/env bash
set -euo pipefail

test -f docs/ui-spec.md

for required in \
  'FocusTarget' \
  'Navigator' \
  'Inspector' \
  'CycleFocus' \
  'footer_shortcuts'; do
  if ! rg -q "$required" docs/ui-spec.md crates; then
    echo "UI contract requirement missing: $required" >&2
    exit 1
  fi
done

echo "UI contract foundation verified"
