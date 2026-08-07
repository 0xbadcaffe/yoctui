#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
case "${1:-}" in
  devtool) cargo test -p yoctui-bitbake devtool_ ;;
  recipetool|bitbake-layers|oe-pkgdata-util|core|advanced) ./scripts/verify-utility-coverage.sh ;;
  *) echo "usage: $0 {devtool|recipetool|bitbake-layers|oe-pkgdata-util|core|advanced}" >&2; exit 2 ;;
esac
echo "utility fixture coverage passed: $1"
