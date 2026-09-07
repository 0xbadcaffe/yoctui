# Current Task

**ID:** OPENBMC-CLI-BUILD-001
**Title:** Fix daemon build generation handshake and rejected-command exit status
**Status:** IN_PROGRESS

Depends on OPENBMC-ENV-001 (DONE). Startup/capability and large-inventory fixes
are complete through v0.1.69. The real Romulus daemon delivered all 4,799 recipes
and nine layers through bounded chunks. Full checks: 1,535 workspace tests,
49 bridge tests, Clippy, formatting, Ruff/mypy and docs pass. See
[OpenBMC validation](testing/openbmc.md).

Two real `yoctui daemon build obmc-phosphor-image` invocations returned
StaleGeneration and exit status zero. `daemon_start_build` currently sends one
request using its attached snapshot generation and returns success for any
CommandResult. Telemetry can advance the journal between attach and command.
Preserve generation checks: bounded refresh/retry only after explicit stale
rejection, never after acceptance or ambiguous I/O. Verify unchanged environment
authority before retrying; rejected/failed outcomes must return nonzero. Tests
must prove no duplicate accepted builds, bounded retry exhaustion and errors.
Daemon's command-processing `continue` is inside its inner receive loop, not
the client-owner loop; do not assume it disconnects rejected clients.

The private OpenBMC v0.1.69 daemon PID 2445464 is idle, inventory loaded. Verify
identity before stopping. Paths and XDG isolation are in the evidence. Source
revision `d4fd7d3f54e88e800c0284b753af68a13aabbef6`, build
`/home/bspguy-dev/src/build-openbmc-romulus`, MACHINE romulus, BitBake 2.19.0.
Do not start the actual image until OPENBMC-TOOLS-001 also passes. Then perform
OPENBMC-LIVE-001 through Yoctui and investigate real outcomes.

Preserve Poky data. Root has about 34 GiB free after compiler checks; clean only
regenerable agent Cargo outputs after saving the tested binary when needed.
New OpenBMC disk guards stop scheduling at 15 GiB / halt at 8 GiB free. A shallow
checkout os-release Git-tag warning remains to inspect during image validation.

Verification: `cargo test -p yoctui daemon_build`, full workspace, strict Clippy,
docs/roadmap, then real image invocation after tool discovery is fixed. Installed
release remains v0.1.64. Full completion still needs fresh source-bound real-Poky
performance evidence; no image-build or release completion is claimed.
