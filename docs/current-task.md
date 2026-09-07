# Current Task

**ID:** OPENBMC-INVENTORY-001
**Title:** Transfer large recipe inventories within bounded bridge frames
**Status:** IN_PROGRESS

Depends on OPENBMC-STARTUP-001 (DONE in v0.1.68). Startup IPC is now responsive
while an owned background scan parses recipes. Full checks: 1,530 workspace
tests, 47 bridge tests, Clippy, formatting and docs pass. Live startup/attach/
shutdown evidence is in [OpenBMC validation](testing/openbmc.md).

New observed defect: Romulus parsing completes, but `list_recipes` emits over
the bridge's 1,048,576-byte line limit. The attached client receives a clear
failure without disconnecting, but no inventory arrives. Add bounded inventory
chunking/pagination with explicit completion, correlation/order checks and
aggregate bounds. Keep legacy bridge compatibility; do not remove frame limits
or silently truncate recipes. Check downstream Workspace/daemon frame limits.
Add failing tests first, then verify real inventory reaches an attached client.

OpenBMC revision `d4fd7d3f54e88e800c0284b753af68a13aabbef6`, source
`/home/bspguy-dev/src/openbmc`, build `/home/bspguy-dev/src/build-openbmc-romulus`,
MACHINE romulus, DISTRO openbmc-openpower, BitBake 2.19.0. The private test daemon
PID 2404826 is idle after reporting the transfer error; verify identity before
stopping it. Private XDG paths are documented in the evidence. No image build
has been accepted yet. Installed release remains v0.1.64.

Next: OPENBMC-CLI-BUILD-001 (bounded stale-generation retries and nonzero rejected
command status), OPENBMC-TOOLS-001 (safe canonicalized PATH directory discovery),
then OPENBMC-LIVE-001: build obmc-phosphor-image through Yoctui and inspect it.
Preserve all user Poky data. Root has about 48 GiB free after validation builds;
agent-generated Cargo outputs can be cleaned after saving the tested binary.
The new OpenBMC config stops scheduling at 15 GiB and halts at 8 GiB free.

Verification: bridge tests, backend tests, full workspace, strict Clippy,
docs/roadmap, real OpenBMC inventory capture. Full completion also requires
fresh source-bound real-Poky performance evidence. Do not claim release or
image-build completion from fixture-only or old-version evidence.
