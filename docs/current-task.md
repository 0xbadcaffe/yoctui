# Current Task

**ID:** CONCEPT-DETAIL
**Title:** Align task error rootfs editor and terminal detail layouts
**Status:** IN_PROGRESS

Checkpoint v0.1.102 tags 3edb883 and is pushed. Implement UI specification
section 45 in layout/render/header/dashboard modules; preserve typed state,
action routing and safe narrow rendering. Update cell goldens and inspect the
six generated PNGs. Required checks: cargo test -p yoctui-ui, formatting,
version policy, raster reproduction and roadmap. Then proceed to CONCEPT-DETAIL
and CONCEPT-VERIFY; final delivery includes workspace tests, Clippy and bridge
tests. Historical source-bound live performance evidence remains separate.

CONCEPT-SHELL is verified: 201 app and 283 UI tests pass with golden updates
disabled; shared mouse geometry, distinct dashboard regions, unavailable/ASCII
dials and raster glyph tests pass. Concept/raster verifier corruption tests
remain enforced. Renderer v2 records exact cell-derived Braille and border
geometry; only generated production PNGs change, never original concepts or
historical live observations. Remaining work is the other five scene layouts
and final baseline/delivery verification.
