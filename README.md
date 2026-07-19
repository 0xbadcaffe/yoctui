# Ratabake

Ratabake is a Rust/Ratatui control frontend for Yocto/BitBake. BitBake remains the metadata and build authority; Ratabake observes it and requests operations.

Run from an initialized environment: `source oe-init-build-env && cargo run -p ratabake -- core-image-minimal`. Use `--backend process` for the Knotty fallback, or `--headless` for scripts and CI. `ratabake doctor` explains environment issues.

The bridge protocol is NDJSON on standard I/O. The included bridge safely negotiates and inspects environment-derived workspace data without parsing configuration as authority; server operations require a compatible live BitBake adapter. See `docs/` for architecture, testing, profiling, protocol, and compatibility details.
