# Portable validation and anonymized host evidence

Validation helpers use `$HOME/src/yoctui-zcu102-2026.1` by default. Set
`YOCTUI_ZCU102_ROOT` to the canonical absolute directory of another initialized
ZCU102 validation checkout. The directory must exist in both the host and the
validation container at the same path. `YOCTUI_ZCU102_CONTAINER` selects the
container for the shell helpers. Keep the existing release binary and vendor
setup prerequisites; these helpers do not create a checkout or container.

The Python inspector checks canonical paths, peer ownership and exact daemon
build authority. The cache-preseed helper additionally checks root ownership,
pinned source identity and BitBake's fetch lock; it never replaces an existing
cache. Its root override does not override the kernel revision, branch or remote.

The ignored OpenBMC defaults smoke test requires `YOCTUI_OPENBMC_NATIVE_BUILD`
and `YOCTUI_OPENBMC_RETAINED_BUILD`. The first identifies the current native
Romulus build without matching symbols; the second identifies the retained
build with matching symbols. Its read-only discovery and all field assertions
are preserved. Normal test runs do not execute this machine-dependent smoke.

## Historical evidence redaction

Historical paths and ownership examples for the development account now use
`build-user`, an anonymized account name. `/home/build-user` in records is a placeholder, not a
runtime dependency or a promise that the recorded file is locally available.
Documentation commands that can be rerun use the current home directory instead.

Terminal text, ANSI captures, cell/style sources, SVGs and affected live raster
images have consistent account redaction. The replacement has the same column
width; styles, geometry, measurements, build outcomes, timestamps, source
revisions and binary identities retain their original meaning. Affected artifact
hashes, manifests and checksum inventories describe the redacted bytes. Original
hashes for uncommitted external evidence remain historical references. This is
privacy redaction of retained observations, not a new live build or certification.

The feat_prod patch increments the coherent workspace version to 0.1.316 because
the repository version policy counts Rust test edits as product changes.
Runtime Rust code is unchanged; renderer goldens and their derived production
images carry the new version identity. Historical live evidence keeps its
recorded release versions and source/binary identities.

Git history remains intact. This branch removes the account from current files;
it does not rewrite ancestor commits or remove copies already published elsewhere.
