#!/usr/bin/env bash
set -euo pipefail

matches="$(rg -n -i 'r[a]tabake' \
  --glob '!target/**' \
  --glob '!Cargo.lock' \
  --glob '!.git/**' \
  . || true)"

if [[ -n "$matches" ]]; then
  printf '%s\n' "obsolete legacy application name found:" >&2
  printf '%s\n' "$matches" >&2
  exit 1
fi
