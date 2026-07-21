# Compatibility

Yoctui requires stable Rust and an initialized Yocto environment for production BitBake control. The bridge negotiates protocol version 1 and reports unsupported server commands explicitly.

The Python bridge localizes BitBake-version selection in `select_adapter`. It recognizes BitBake major version 1 as the legacy adapter family and major versions 2 and later as the modern family. Missing BitBake modules use an environment-only adapter for protocol and test operation; this is not a build-control adapter. Malformed and pre-1 version values are rejected with `unsupported_bitbake` before command processing.

This repository has mocked-module coverage for a BitBake 2.8-style version, including native-style task event objects delivered by an optional, non-blocking `drain_events()` connection hook. When a connection provides typed `list_recipes(filter)`, `list_layers()`, or `get_variable(name, recipe)` methods, the bridge presents those results without parsing metadata itself; otherwise it uses the environment-only compatibility data. The bridge normalizes build, task, progress, and log records while retaining unknown events as diagnostics rather than guessed behavior. It does not yet claim a tested live Yocto/BitBake build-control combination; actual server APIs must be validated against a supported Yocto release before production use.
