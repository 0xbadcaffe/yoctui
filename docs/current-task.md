# Current Task

**ID:** OPENBMC-PKGDATA-001
**Title:** Resolve installed package names through bounded Yocto runtime-reverse metadata
**Status:** IN_PROGRESS

The real image SUCCEEDED through Yoctui: obmc-phosphor-image, Romulus,
6812/6812, image job 1 exit 0. The v0.1.84 production package screen exposed
40 missing package metadata records despite readable runtime-reverse mappings.
For example libblkid1 maps to ../runtime/util-linux-libblkid. The adapter only
tries runtime/<installed-name>. Register this separately from the attached
client's missing recipe-scoped IMAGE_ROOTFS lookup (OPENBMC-ROOTFS-SOURCES-001).
OPENBMC-LIVE-001 waits for both repairs and actual UI rechecks.

## Outcome and boundaries

Resolve manifest package names using authoritative generated runtime-reverse
metadata, retaining the installed identity and exact recipe/size/file fields.
Accept only contained mappings to regular runtime files, with existing byte,
line, time and cancellation bounds. Do not generally enable symlink traversal,
guess names, double-count aliases, or hide missing/malformed metadata.
Cover direct, renamed/scoped fields, conflict, missing/dangling/escaping/looped
mappings and normal cancellation/bounds. Keep the current layout unchanged.

Files: crates/yoctui-bitbake/src/rootfs.rs and relevant adapter tests;
docs/rootfs-composition.md, docs/architecture.md, docs/ui-spec.md,
docs/testing/openbmc.md, registry, roadmap and implementation status.

Verification: focused failing-first rootfs tests; cargo test --workspace
--all-features; cargo clippy --workspace --all-targets --all-features -- -D warnings;
cargo fmt --all --check; python3 -m pytest bridge/tests;
./scripts/check-docs.sh; ./scripts/verify-roadmap.sh; production image UI recheck.
Every commit bumps the patch version, goldens and six production rasters.
Run all Cargo commands sequentially with one worker, dev/test debug=0,
incremental disabled and RUST_TEST_THREADS=2.

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

v0.1.84 registration baseline passes: 1,580 workspace tests/doc-tests,
52 bridge tests, strict Clippy, fmt, docs, rasters, roadmap and version policy.
All 17 golden diffs are version-only; no runtime source changes.
After the two repairs, resume OPENBMC-LIVE-001 and the full completion gate,
which still requires fresh source-bound real-Poky performance evidence.
