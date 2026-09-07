# Current Task

**ID:** OPENBMC-TOOLS-001
**Title:** Discover tools through canonicalized initialized PATH directories
**Status:** IN_PROGRESS

Depends on OPENBMC-ENV-001 (DONE). Capability, startup, inventory and CLI fixes
are complete through v0.1.70. All 1,541 workspace tests, 49 bridge tests, Clippy,
formatting, Ruff/mypy and docs pass. The real CLI recovered stale generation
and rejected a deliberately invalid target with exit 1 and no jobs created.
See [OpenBMC validation](testing/openbmc.md).

OpenBMC scripts is a directory symlink to upstream-layers/openembedded-core/scripts.
Both lexical and canonical devtool --help work, but discover_executable rejects
noncanonical PATH directories and incorrectly reports the tool absent. Require
an absolute initialized PATH directory, canonicalize that directory, then join
the requested tool basename. Preserve existing safe basename-sensitive sibling
file aliases, executable/file checks and escaping/dangling/relative-path
rejection. Add tests first; then verify real Devtool, Recipetool and pkgdata-tool
evidence from a restarted private daemon. Do not widen the final-file alias
safety policy or strip argv[0] aliases such as bitbake-dumpsig.

Private OpenBMC v0.1.69 daemon PID 2445464 is idle, all 4,799 recipes/nine layers
loaded. Verify identity before stopping. Private XDG paths are in the evidence.
Source revision d4fd7d3f54e88e800c0284b753af68a13aabbef6, source
/home/bspguy-dev/src/openbmc, build /home/bspguy-dev/src/build-openbmc-romulus,
MACHINE romulus, DISTRO openbmc-openpower, BitBake 2.19.0.

After this fix, execute OPENBMC-LIVE-001 through Yoctui: build
obmc-phosphor-image, inspect lifecycle/logs/outcomes, packages and rootfs, and
register/fix real regressions. No image build has been accepted yet.
Preserve all Poky data. Root has about 26 GiB free after compiler checks; save
the tested candidate and clean regenerable agent Cargo workspace outputs before
the image build. New OpenBMC guards stop scheduling at 15 GiB / halt at 8 GiB.
Inspect the shallow checkout's os-release Git-tag warning during validation.

Verification: daemon compatibility tests, full workspace, strict Clippy, bridge
and docs/roadmap checks, and real tool identity/capability capture. Installed
release remains v0.1.64. Full completion still requires fresh source-bound
real-Poky performance evidence and all remaining tasks, not fixture-only claims.
