#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

if [[ "${YOCTUI_LIVE_COMPLETE:-0}" == "1" ]]; then
  ./scripts/test-live-next-generation-ui.sh
fi
python3 ./scripts/finalize-live-m22-concept-evidence.py
./scripts/verify-live-m22-concept-evidence.sh
