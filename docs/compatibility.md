# Compatibility

Yoctui requires stable Rust and an initialized Yocto environment for production BitBake control. The bridge negotiates protocol version 1 and reports unsupported server commands explicitly.

The Python bridge localizes BitBake-version selection in `select_adapter`. It recognizes BitBake major version 1 as the legacy adapter family and major versions 2 and later as the modern family. Missing BitBake modules use an environment-only adapter for protocol and test operation; this is not a build-control adapter. Malformed and pre-1 version values are rejected with `unsupported_bitbake` before command processing.

This repository has mocked-module coverage for a BitBake 2.8-style version. It does not yet claim a tested live Yocto/BitBake build-control combination. BitBake event-object adaptation remains intentionally localized to the Python bridge; unknown events are retained as diagnostics rather than guessed.
