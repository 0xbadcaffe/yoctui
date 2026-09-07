# README header

The README uses the user-supplied white circuit/block Yoctui wordmark on a
near-black background, with the supplied subtitle and tagline. The banner is
compact and has descriptive alternative text. Existing feature documentation,
commands, screenshot evidence and compatibility qualifications remain intact.

Badges and navigation are HTML links outside the image so they are clickable
and can wrap on narrow displays. CI links to the real `ci.yml` workflow;
crates.io uses a dynamic published-version badge, not the checkout version.
Coverage links to the documented verification gates without claiming an
aggregate percentage or Codecov integration. Rust is labeled stable because
there is no declared numeric MSRV. MIT, Linux and the operator guide are backed
by repository configuration. Community links to GitHub issues; no Discord
invite is configured. Add Discord only after an actual project invite is given.

No application UI, component boundary, sampling rate or protocol changes are
part of this documentation task. The version-only terminal fixture refresh is
required by the repository's per-commit version policy, not a new UI layout.

## Artwork provenance

Source: the owner's attached Photo 1.jpg, supplied on 2026-09-08.
The built-in image-generation tool produced the header-only PNG at
`docs/media/yoctui-header.png`; the original attachment is unchanged.

Edit prompt: preserve the white pixel/block circuit-style YOCTUI wordmark,
double-line outlines and near-black background; retain exactly "TUI for the
Yocto Project" and "visualize · build · inspect · develop · debug · all in your
terminal"; remove all badges and navigation; trim the large empty margins into
a compact approximately 3:1 banner with balanced padding; add no new elements.

## Verification

`./scripts/test-readme-quickstart.sh` checks header links, local targets,
alternative text, PNG signature/dimensions and unchanged operator coverage
without network or a Rust rebuild. Existing raster, roadmap, version and
formatting checks cover the mechanical version refresh. This does not replace
full release verification or real-build performance evidence.
