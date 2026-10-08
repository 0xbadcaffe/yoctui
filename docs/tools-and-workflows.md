# Tools and workflows

Availability comes from the initialized build's daemon capability snapshot.
Unknown/missing tools remain visible with reasons. Typed forms and expert argv
use shell-free review; mutations require confirmation. Host PATH alone is insufficient.

| Tool family | Workflow |
| --- | --- |
| `oe-init-build-env`, `oe-setup-builddir`, `oe-buildenv-internal` | Environment setup/internal information. |
| `bitbake` | Reviewed targets/tasks/options, events, cancellation. |
| `devtool`, `recipetool` | Recipe/source/layer forms; capability-gated expert arguments. |
| `bitbake-layers` | Layer queries and confirmed mutations. |
| `oe-pkgdata-util` | Generated package metadata. |
| `bitbake-getvar`, `bitbake-dumpsig`, `bitbake-diffsigs`, `dumpsig`, `diffsigs`, `whatchanged` | Variables/signature comparisons. |
| `runqemu`, `wic`, `runqemu-extract-sdk` | Artifact-bound launch/creation/device review. |
| `oe-find-native-sysroot`, `oe-run-native` | Reviewed native environment/argv. |
| `kas` | Installed expert checkout/build/shell/lock, network/mutation review. |
| `oe-selftest`, `bitbake-selftest`, `testimage`, `testsdk`, `ptest`, `resulttool` | Tests, results, comparison, JUnit. |
| CVE/SPDX/SBOM helpers | Exact report paths, explicit partial results. |
| `yocto-check-layer`, `yocto-layer`, `yocto-bsp`, `yocto-kernel` | Version-aware checks/expert operations. |
| `sstate-cache-management.sh` | Exact cleanup candidates and destructive review. |
| `buildhistory-diff`, `build-compare`, locked signatures | Exact source/cache evidence. |
| `oe-git-archive`, `create-pull-request`, `send-pull-request` | Local review, separate network confirmation. |
| PR/hash services, `toaster`, `pybootchartgui` | Information/managed diagnostics. |
| `bitbake-worker`, `bitbake-prserv`, `bitbake-hashserv` | Internal; not user-launchable. |

Adapters need capability/action entries, bounded shell-free runner tests, cancellation,
and this table. verify-utility-coverage.sh --catalog-only checks coverage;
fixtures do not establish every [live workflow](compatibility.md).
