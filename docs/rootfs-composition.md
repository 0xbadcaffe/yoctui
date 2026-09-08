# Rootfs Composition Evidence

## Offline udev rules

In Images, select an artifact and load rootfs composition, then press `6` or
cycle Tab to **udev rules**. The selected image's reported `IMAGE_ROOTFS` is
the only authority. Up/Down, Page Up/Down, Home/End select rules; `[`/`]`
scroll the preview, `r` refreshes, and Enter opens the rootfs explorer.

The inventory retains vendor, runtime, and administrative `.rules` files,
including files overridden by a higher-priority same-name file and `/dev/null`
masks. Image-absolute symlinks resolve inside IMAGE_ROOTFS; host files and
devices are never substituted. File selection does not mean any rule matched
a live device. Rules and their `RUN`/`IMPORT` commands are never executed.

The scan allows at most 4,096 rules and 65,536 visited directory entries, with
8 KiB previews and the shared cancellation/deadline. Unreadable/broken entries
remain explicit; scan limits report Partial, and a cleaned rootfs is Unavailable.
Preview truncation is labeled independently. These are generated image files;
changes made through the explorer may be overwritten by a later BitBake task.

Search directories and precedence follow the upstream
[udev rules documentation](https://github.com/systemd/systemd/blob/main/man/udev.xml),
including the legacy `/lib/udev/rules.d` vendor location used by older images.

Yoctui can open the exact BitBake-reported `IMAGE_ROOTFS` as a lazy file tree,
preview bounded text files, and edit a selected file without booting the image.
This requires the staged work directory to still exist; a deploy-only ext4,
Wic, or compressed artifact is not mounted or extracted implicitly.

The Images systemd view lists service units and enablement-link evidence. The
system D-Bus view combines activation descriptors, `BusName=` declarations,
`SystemdService=` links, and policy-file references. These views describe the
files in the staged image. They cannot report live unit state, current bus-name
owners, or activation results before boot.

Rootfs composition is an Images subview correlated to one exact image,
machine, build, and artifact identity. It presents two independent evidence
sources and never combines their totals.

## Installed-package authority

Installed packages come from the selected image's exact image manifest plus
authoritative runtime pkgdata. The package view can group and drill into those
records, including a bounded, inspectable `Other` group. It does not infer
installation from recipe metadata, deploy filenames, or filesystem contents.

Installed names use Yocto's generated runtime-reverse index when present.
Only a single relative link of the form ../runtime/<original-package> is
accepted, with non-symlink contained index/runtime directories and a regular
runtime record. The record's PKG field must corroborate a renamed installed
identity; scoped size/file fields use the original record name. A missing
index permits direct-name metadata, but malformed, conflicting or dangling
mappings remain explicit Partial evidence, not a guessed fallback. Duplicate
manifest names are counted once. General file/rootfs symlink rules are unchanged.

Runtime pkgdata is read incrementally under separate per-file, per-line, and
aggregate byte limits. Current recipe-scoped fields such as
`PKGSIZE:<package>` and `FILES_INFO:<package>` are parsed without retaining the
whole file or JSON file map in memory. If one installed package exceeds a
safety bound, installed-package composition remains Partial with an explicit
limitation; the entire Rootfs screen does not become Failed.

If the manifest or pkgdata is absent, stale, outside the active build, or does
not correlate to the selected identity, the view is Unavailable with a reason.
A successful image build does not imply that every optional pkgdata operation
or another image has evidence.

## Logical-filesystem authority

Filesystem composition is optional. It is acquired only from BitBake's exact
recipe-scoped `IMAGE_ROOTFS`, after canonical build containment and symlink-root
checks. Traversal does not follow symlinks, deduplicates hard-linked regular
file bytes, distinguishes special files, and is bounded by entry count, depth,
input bytes, accounted bytes, elapsed time, and cancellation.

Missing or cleaned work state is Unavailable. A reached bound is Partial and
names the limitation. Package-reported sizes never substitute for filesystem
bytes, and a Partial logical view is never presented as a complete total.
With `rm_work`, it is normal for installed-package evidence to remain while
the logical filesystem reports `Unavailable (cleaned)`.

## Presentation and accessibility

Wide color layouts may render a pie chart, but the same view always includes
exact bytes, percentages, counts, labels, and membership. Medium layouts use
bars and tables. Narrow, ASCII, no-color, high-contrast, and reader-oriented
layouts use deterministic tables and trees. Selection, grouping, scrolling,
and totals remain model-owned; the chart is never the sole evidence.

Rootfs and Wic artifacts are storage images rather than raster pictures, so
Yoctui does not probe them for graphical preview. Exact metadata and Rootfs
composition are the terminal-safe fallback.

## Recorded live boundary

On 2026-09-08 the v0.1.85 production client inspected the successfully built
Romulus obmc-phosphor-image manifest: all 228 packages have available generated
metadata, 81062873 installed bytes and 1778 files. The bounded runtime-reverse
repair removes 40 previously missing renamed-package records. The separate
attached IMAGE_ROOTFS source-query defect remains explicitly unavailable under
OPENBMC-ROOTFS-SOURCES-001; retained filesystem existence alone is not UI
authority. See [OpenBMC evidence](testing/openbmc.md).

An opt-in live adapter regression also scans the exact deployed manifest and
machine-scoped pkgdata directory from a completed image. On 2026-09-01 it
validated the Wrynose 6.0.2 `core-image-kernel-dev` output, including the valid
1.1 MiB `kernel-devsrc` runtime-pkgdata record that originally exposed the
legacy 256 KiB limit. This is operational regression evidence, not a broader
release-support claim.

The 2026-08-27 M21 live run built `core-image-minimal` for `qemux86-64` with
Poky 5.2.4 / BitBake 2.12.1 on Ubuntu 24.04.4. It recorded 38 manifest packages
and 14,995 pkgdata files. The build enabled `rm_work`, so the exact logical
root was honestly recorded as `unavailable_cleaned`; no filesystem total is
claimed. Re-run the evidence verifier described in [Testing](testing.md), and
consult [Compatibility](compatibility.md) before generalizing that observation
to another release or host.
