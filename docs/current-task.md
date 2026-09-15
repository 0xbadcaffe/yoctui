# Current Task

**ID:** README-FLAMEGRAPH-001
**Title:** Restore the validated Flamegraph report to the README
**Status:** DONE

Completed in v0.1.115. The README embeds the checked real-perf SVG, links its
machine-readable summary and reproduction path, and reports the capture version,
date, workload, userspace sample count, checksum and zero-unresolved result. It
also states that the retained v0.1.64 capture is historical because application
sources have changed.

The README test derives displayed facts from the summary and binds its weighted
event total to the SVG. The Flamegraph workload/validator test, full all-features
workspace suite, strict Clippy, 52 bridge tests, documentation, version/layout,
roadmap and both deterministic screenshot checks pass.
