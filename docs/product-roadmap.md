# Development priorities

Current behavior is defined by [UI specification](ui-spec.md),
[architecture](architecture.md), and [operator guide](operator-guide.md).
Design direction is in [workbench design](workbench-design.md).

## Open validation and integration work

The following capabilities still require the indicated work. This is a backlog,
not an autonomous agent queue or a claim of product completeness. Work should
follow the current developer request.

| Area | Remaining work | Constraint |
| --- | --- | --- |
| DEMO-DOCS-001 | Refresh README real screenshots measured flamegraph report and operator runbook | Requires explicit scope and representative validation |
| DEMO-NATIVE-PERSISTENCE-001 | Prepare durable native OpenBMC daemon startup after laptop login and reboot | Requires explicit scope and representative validation |
| DEMO-NATIVE-POSTREBOOT-001 | Verify accepted native demo after coordinated laptop reboot and login | Requires explicit scope and representative validation |
| DEMO-INSTALL-LIVE-001 | Install optimized release and rehearse all OpenBMC demo screens and sessions | Requires explicit scope and representative validation |
| DEMO-DOCS-JSON-STREAM-001 | Validate full Doctor JSON without the OS argument-size limit | Requires explicit scope and representative validation |
| DEMO-GITHUB-CI-001 | Fix remaining GitHub CI failures after local demo release verification | Requires explicit scope and representative validation |
| XILINX-ZCU102-BUILD-001 | Build a real ZCU102 PetaLinux image through the Yoctui daemon | External prerequisite or live evidence remains unavailable |
| XILINX-ZCU102-QEMU-GDB-001 | Validate ZCU102 QEMU boot and live kernel debugging inside Yoctui | Requires explicit scope and representative validation |
| KERNEL-INSTRUMENTATION-LIVE-001 | Verify preset retention and diagnostics on a matching instrumented kernel | External prerequisite or live evidence remains unavailable |
| KGDB-SERIAL-LIVE-001 | Verify KGDB attach backtrace resume and re-entry on a real board | External prerequisite or live evidence remains unavailable |
| M67-LIVE-EVIDENCE-001 | Supply current-source real-Poky release performance evidence | External prerequisite or live evidence remains unavailable |

## Workspace principles

BitBake and Devtool remain authoritative for metadata and workspace changes.
Use typed events and daemon-owned capability snapshots. Renderers consume model
state rather than parsing process text. Unknown progress remains unknown.
Destructive operations require an exact preview and explicit confirmation.
Preserve active builds when navigating and use the shared focus, dialog, and
footer models. Live acceptance requires actual supported environments; fixtures,
recorded screenshots, or restarted services cannot establish board, reboot, or
current-source performance certification.
