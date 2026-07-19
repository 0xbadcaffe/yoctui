# Ratabake

Ratabake is a Rust/Ratatui control frontend for Yocto/BitBake. BitBake remains the metadata and build authority; Ratabake observes it and requests operations.

Run from an initialized environment: `source oe-init-build-env && cargo run -p ratabake -- core-image-minimal`. Use `--backend process` for the Knotty fallback, or `--headless` for scripts and CI. `ratabake doctor` explains environment issues.

The bridge protocol is NDJSON on standard I/O. The included bridge safely negotiates and inspects environment-derived workspace data without parsing configuration as authority; server operations require a compatible live BitBake adapter. See `docs/` for architecture, testing, profiling, protocol, and compatibility details.

Configuration is read from `$XDG_CONFIG_HOME/ratabake/config.toml` (or `~/.config/ratabake/config.toml`). CLI flags override `RATABAKE_*` environment variables, which override the configuration file, which overrides built-in defaults. Supported values include `backend`, `build_dir`, `log_retention_entries`, `log_retention_bytes`, `refresh_ms`, `default_target`, and `color`.
