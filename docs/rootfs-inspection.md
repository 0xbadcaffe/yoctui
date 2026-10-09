# Rootfs inspection

Images → select an artifact → package/filesystem view. Inspection is read-only
unless you explicitly edit a file; it does not boot or mount the deployed image.
Numbered views: 1 artifacts, 2 packages, 3 filesystem, 4 systemd, 5 D-Bus, 6 udev.
Prefer lasting recipe/layer edits; generated rootfs files can be replaced by a build.

## Installed packages

Exact image manifest plus machine-scoped PKGDATA_DIR supplies package membership,
bytes, and file counts. Runtime names map to build-package metadata; missing data
stays unavailable. Package totals are separate from filesystem bytes.

When the selected deployed manifest and workspace pkgdata are available, the pie
chart and package list load first, without waiting for BitBake metadata or the
filesystem/service scan. The inspector labels details still loading; those views
fill in when the same image/daemon generation finishes. Refresh reacquires data;
stale or cancelled replies cannot replace the current image.

## Logical filesystem

The recipe's exact IMAGE_ROOTFS supplies filesystem content. Instance/generation/
image checks reject stale replies. Lazy traversal is bounded, cancellable, and
contained; symlinks/special files cannot expose host data. Limits/unreadable entries
produce Partial. rm_work may leave packages available but rootfs Unavailable (cleaned).

Files opens before the complete filesystem/service inventory finishes. Matching
build-generated `.testdata.json` beside the exact deployed manifest can supply
IMAGE_ROOTFS without starting BitBake queries; recipe, machine, image name and
build containment must match. Missing/invalid/cleaned metadata falls back to the
normal query. Finishing the inventory preserves the folder and selection.
This also works for MTD artifacts with a retained build rootfs: it does **not**
mount or unpack a flash image, or claim to inspect bytes changed inside that image.

Arrows or h/j/k/l navigate; Enter expands/previews; Left returns; `.` shows hidden
files; `/` searches; `e` edits. Rows report on-disk ownership/permissions, not fakeroot
ownership. Saves check conflicts/containment; later BitBake tasks may replace edits.

## Services and rules

| View | Reads |
| --- | --- |
| systemd | Installed units, descriptions, masks, image-local enablement links. |
| System D-Bus | Activation/policy files, users, executables, associated units. |
| udev (`6` or Tab) | Installed rules, overrides, /dev/null masks; `[`/`]` preview, `r` refresh. |

Enter (or `e`) opens the selected systemd, D-Bus or udev file in the built-in
source editor without switching to Files. Right opens the rootfs explorer.
udev previews and editors highlight keys, operators, quoted values and comments;
masked or unresolved rules cannot be opened as editable text.

These are offline files, not running service/bus/device state. udev never executes
RUN/IMPORT: at most 65,536 entries, 4,096 rules, and 8 KiB preview; truncation is labelled.

## Layout

Wide charts pair exact bytes/percentages/counts; narrow/ASCII/no-color/accessibility
modes use equivalent tables/bars/trees. At 80x24, insufficient chart space gives the
full-height table. Storage-image suffixes do not imply raster preview support.

## Validation boundary

Fixtures cover mapping, bounds, stale replies, filesystem cases, and layout.
[Live tests](testing.md#live-yocto-validation) need exact manifest/pkgdata and optional
retained IMAGE_ROOTFS; an offline scan does not certify guest/board boot.

The offline systemd view was informed by the MIT-licensed
[systemd-manager-tui](https://github.com/Matheus-git/systemd-manager-tui) by Matheus-git;
Yoctui uses its own parser for unbooted images.
