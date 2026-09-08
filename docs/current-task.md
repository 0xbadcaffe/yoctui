# Current Task

**ID:** OPENBMC-ROOTFS-SOURCES-001
**Title:** Acquire exact image rootfs sources for daemon-attached clients
**Status:** IN_PROGRESS

OPENBMC-PKGDATA-001 is DONE in v0.1.85. Four new regressions, 13 focused rootfs
tests, 281 UI tests, 1584 workspace tests/doc-tests, 52 bridge tests, strict
Clippy, production build, fmt, docs, rasters, roadmap and version policy pass.
All 17 golden diffs are version digits only. The actual production screen
reports all 228 installed packages available, 81062873 bytes and 1778 files;
all 40 missing metadata records are resolved. No daemon restart or rebuild.
Captured tested candidate /home/bspguy-dev/.local/state/yoctui-v85-validated.YhRRZA/yoctui,
SHA-256 cf19348c978d8e3810874d49cad4875aa5a1194bce0dd21871de1699f32bd9b0.

## Outcome and boundaries

The retained image rootfs exists, but attached clients instantiate ProcessBackend
whose get_variable returns None. Recipe-scoped IMAGE_ROOTFS queries never reach
initialized BitBake; manifest/pkgdata fallbacks mask part of the missing path.
Bridge get_rootfs_sources already exists. Supply bounded exact-image source
metadata through an appropriate daemon-owned read-only path. Bind recipe/image
and request identity, current instance/generation/capability and workspace.
Keep input responsive while metadata is acquired; preserve honest
absent/cleaned, stale/disconnected, failed and timed-out states. Do not guess
workdir paths, require local attach environment, mount images or rebuild.

Update authoritative UI/architecture contracts before intentional behavior.
Files include daemon protocol/CLI metadata and client routing, existing bridge
API as needed, relevant app/model tests and docs/testing/openbmc.md.
Cover normal exact-image sources, legacy/missing support, stale/replaced
authority, malformed/absent/cleaned paths, query failure/cancellation/timeout,
fake-process integration and production real rootfs/services screen recheck.
Retain existing source/path containment and scan limits.

Verification: focused failed-first protocol/daemon/client tests; cargo test
--workspace --all-features; cargo clippy --workspace --all-targets --all-features
-- -D warnings; cargo fmt --all --check; python3 -m pytest bridge/tests;
./scripts/check-docs.sh; ./scripts/verify-roadmap.sh; real rootfs UI recheck.
Every commit bumps patch version, 17 goldens and six rasters. Run Cargo checks
sequentially, one worker, dev/test debug=0, no incremental and RUST_TEST_THREADS=2.
After this repair resume OPENBMC-LIVE-001, then verify-completion.sh; fresh
source-bound real-Poky performance evidence is still required.

## Preserved live authority

- Source /home/bspguy-dev/src/openbmc, clean d4fd7d3f54e88e800c0284b753af68a13aabbef6
- Build /home/bspguy-dev/src/build-openbmc-romulus; MACHINE romulus;
  DISTRO openbmc-openpower; BitBake 2.19.0
- Private daemon PID 2310652, instance 4832e5d8285444a5445264dae2900239;
  read daemon.json before any lifecycle action
- Binary /home/bspguy-dev/.local/state/yoctui-v84-validated.Gh0uoK/yoctui
- SHA-256 c1fd3f3ccd2c269d3c7d11d35e1f7b69deb1a6c5ff0a553b33621c0aa9f0d221
- Private XDG config/state under
  /home/bspguy-dev/.local/state/yoctui-openbmc-validation;
  runtime /run/user/1000/yoctui-openbmc-validation
- Old image daemon v0.1.76 PID 1302830 stopped normally after success.
  v0.1.84 recovered image job 1 and failed historical job 2 unchanged;
  actual llvm-native/stdplus listtasks used job 3, exited 0, 2/2.
- Actual queued/start/completed PNs match, timestamps survive compaction,
  and two fresh production attachments retain elapsed 00:00:01 for the
  measured 1814-ms build. No active/waiting ghost rows remain.

Image/artifact and lifecycle evidence is under artifacts/live-openbmc/romulus.
The generated 32 MiB MTD and XZ SquashFS match the retained rootfs; 228 manifest
packages and 274 service files were inspected. The upstream render sysusers
warning was traced, not patched/suppressed; runtime impact is untested.
No firmware boot or completion-gate success is claimed. Normal installed
release stays v0.1.64. Preserve all Poky data and all four user-owned untracked
retry-active-dashboard captures. No image rebuild is needed for metadata fixes.
