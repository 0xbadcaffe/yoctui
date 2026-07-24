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

for required in \
  'model_action_from_backend_event' \
  'RecipesLoaded' \
  'LayersLoaded' \
  'VariableLoaded' \
  'RecipeSourcesLoaded' \
  'DependenciesLoaded' \
  'LayerRelationshipsLoaded'; do
  if ! rg -q "$required" crates/yoctui-model crates/yoctui-app; then
    echo "typed backend boundary requirement missing: $required" >&2
    exit 1
  fi
done

if rg -q 'BackendEvent|yoctui_protocol|serde_json' crates/yoctui-ui; then
  echo "UI renderer must consume typed model state, not backend/protocol data" >&2
  exit 1
fi

echo "UI contract foundation verified"
