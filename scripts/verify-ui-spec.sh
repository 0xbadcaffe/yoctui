#!/usr/bin/env bash
set -euo pipefail

test -f docs/ui-spec.md

for required in \
  'FocusTarget' \
  'CycleFocus' \
  'Theme' \
  'AnimationSpeed' \
  'OpenCommandPalette'; do
  if ! rg -q "$required" crates/yoctui-model crates/yoctui-app; then
    echo "UI contract requirement missing: $required" >&2
    exit 1
  fi
done

for required in 'Navigator' 'Inspector' 'footer_shortcuts' 'task_activity'; do
  if ! rg -q "$required" crates/yoctui-ui; then
    echo "UI renderer requirement missing: $required" >&2
    exit 1
  fi
done

echo "UI contract foundation verified"
