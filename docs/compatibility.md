# Compatibility

Yoctui requires stable Rust and an initialized Yocto environment for production BitBake control. The bridge negotiates protocol version 1 and reports unsupported server commands explicitly.

The Python bridge localizes BitBake-version selection in `select_adapter`. Installed BitBake packages use the supported `bb.tinfoil.Tinfoil` client API for workspace inspection, effective variables, parsed recipes, configured layers, build submission, native event delivery, cancellation, and shutdown. It recognizes BitBake major version 1 as the legacy adapter family and major versions 2 and later as the modern family. Missing BitBake modules use an environment-only adapter for protocol and test operation; this is not a build-control adapter. Malformed and pre-1 version values are rejected with `unsupported_bitbake` before command processing.

The following live combination was observed on 2026-07-24 with `scripts/verify-live-bitbake.sh`:

- backend: Python bridge using BitBake Tinfoil
- BitBake: `2.19.0`
- distribution: `poky`
- release: `6.0.99+snapshot-a4eb7bc2a750f76d9772eb88b7afb2b801bd1250`
- machine: `qemux86-64`
- normal smoke operation: `base-files:do_listtasks`
- cancellation target: `core-image-minimal`

That run exercised real workspace inspection, MACHINE lookup, recipe and layer inventories, parse progress, task and log events, normal completion, cancellation, and bridge shutdown. This is an observed development snapshot, not a claim that every BitBake 2.x or Poky snapshot is supported.

The repository also retains mocked-module coverage for older adapter shapes. Mocked tests prove adapter and framing logic only; they are never counted as live compatibility evidence. Unknown native events remain visible as diagnostics rather than being guessed.
