# Yocto utility capability catalog

The catalog is intentionally conservative: availability is detected from the
initialized environment and unsupported entries remain visible with a reason.
Common operations are typed workflows; expert operations use validated argv
preview and never shell interpolation. `live` means compatibility evidence is
required before a release claim; `fixture` means deterministic adapter tests.

| Utility / family | Classification | Safety and evidence policy |
| --- | --- | --- |
| `oe-init-build-env`, `oe-setup-builddir`, `oe-buildenv-internal` | informational/internal | Environment capabilities only; never arbitrary child jobs; live Poky required |
| `bitbake` targets, tasks, `-f`, `-g`, env | typed workflow | Typed target/task forms, preview, cancellation, bounded job output; live bridge evidence |
| `devtool` | typed workflow + expert argv | Recipe/workspace forms for common lifecycle; exact identity, confirmation, fixture coverage and version-aware live evidence |
| `recipetool` | typed workflow + expert argv | Source/layer destination forms; protect existing files and refresh inventory |
| `bitbake-layers` | typed workflow + expert argv | Read-only queries inspectable; mutations previewed and confirmed |
| `oe-pkgdata-util` | typed workflow + expert argv | Packages navigation; explicit unavailable state when pkgdata is absent |
| `bitbake-getvar`, `bitbake-dumpsig`, `bitbake-diffsigs`, `dumpsig`, `diffsigs`, `whatchanged` | typed workflow | Exact variable/signature identity and bounded reports |
| `runqemu`, `wic`, `runqemu-extract-sdk` | typed workflow | Artifact-bound launch/device confirmation; no unrestricted executable strings |
| `oe-find-native-sysroot`, `oe-run-native` | typed workflow | Capability detection and shell-free argv; native environment is displayed |
| `kas` (`checkout/build/shell/lock`) | expert argv | Only when installed; network and mutation warnings are mandatory |
| `oe-selftest`, `bitbake-selftest`, `testimage`, `testsdk`, `ptest` | typed workflow | Bounded sessions, cancellation, result navigation, live evidence per release |
| `resulttool`, JUnit import/export | typed workflow | Typed comparison and bounded artifacts; fixture tests do not claim live Yocto |
| CVE/SPDX/SBOM helpers | typed workflow | Capability-aware reports, exact paths, explicit partial state |
| `yocto-check-layer`, `yocto-layer`, `yocto-bsp`, `yocto-kernel` | expert argv | Version-aware capability menu; destructive/network operations require confirmation |
| `sstate-cache-management.sh`, cleanup/wipe tools | typed workflow | Candidate preview, bounded roots, destructive confirmation |
| `buildhistory-diff`, `build-compare`, locked signatures | typed workflow | Exact repository/cache identity and retained reports |
| `oe-git-archive`, `create-pull-request`, `send-pull-request` | typed workflow | Local preview first; network push is a separate confirmation |
| PR/hash services, `toaster`, `pybootchartgui` | informational/managed service | Diagnostics and managed lifecycle only; never unrestricted one-key launches |
| `bitbake-worker`, `bitbake-prserv`, `bitbake-hashserv` | intentionally excluded | Internal workers/servers are not user-launchable utilities |

New adapters must add a capability entry, typed model/action, shell-free
runner tests, bounded output/error handling, cancellation, and this table.
`./scripts/verify-utility-coverage.sh --catalog-only` enforces that every
required registry utility is represented in this document.
