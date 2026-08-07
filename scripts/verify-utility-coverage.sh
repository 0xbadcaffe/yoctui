#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
catalog="$repo_root/docs/utility-catalog.md"
test -s "$catalog"
required=(oe-init-build-env bitbake devtool recipetool bitbake-layers runqemu wic kas oe-pkgdata-util bitbake-getvar bitbake-diffsigs bitbake-dumpsig oe-find-native-sysroot sstate-cache-management.sh buildhistory-diff yocto-check-layer yocto-layer yocto-bsp yocto-kernel pybootchartgui toaster resulttool oe-selftest bitbake-selftest)
for utility in "${required[@]}"; do
  grep -Fq "\`$utility\`" "$catalog" || { echo "utility missing from catalog: $utility" >&2; exit 1; }
done
if [[ "${1:-}" != "--catalog-only" ]]; then
  grep -Fq 'shell-free' "$catalog"
  grep -Fq 'confirmation' "$catalog"
fi
echo "utility catalog coverage passed"
