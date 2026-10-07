# OpenBMC live integration

## Workspace setup

Use an existing complete OpenBMC checkout and a supported configured machine.
Initialize its environment with the vendor-supported setup wrapper. Check the
selected source/build paths and daemon workspace before attaching; do not
retarget an existing daemon or stop unrelated builds. Opening Yoctui does not
start an image build. See [operator guide](../operator-guide.md).

## Validation boundaries

The recorded Romulus investigation used BitBake 2.19.0. Direct environment and
capability probes remain separate from release support: a successful probe or
image build does not certify firmware boot, board behavior, or every workflow.
Use the [compatibility matrix](../compatibility-matrix.md) and current direct
evidence before making a support claim.

Before live testing, check disk and memory capacity, host tools, the effective
MACHINE, initialized build directory and exact daemon/binary identity. Treat
missing or mismatched metadata, symbols and capability generations as
unavailable. Do not use a host file or another build's cache as target authority.

## Representative checks

- Doctor validates bridge protocol and reports the exact build identity.
- A reviewed supported image build streams real task state, completion and logs.
- Disconnect/reconnect preserves daemon-owned jobs without resubmitting work.
- Package inspection uses the deployed manifest and machine-scoped pkgdata.
- RootFS queries use the exact logical image root, target account files and
  Pseudo evidence; stale or missing sources stay visibly unavailable.
- QEMU/kernel debugging requires matching boot/rootfs/symbol artifacts. A
  successful offline query does not establish guest boot or physical-board safety.
- Cancel or stop only processes owned by the explicit validation session.

## RootFS regression evidence

The retained Romulus manifest/pkgdata observation contains 228 installed
packages, 81,062,873 installed bytes and 1,778 files. Runtime-to-build package
mapping resolves renamed packages instead of assuming equal identities. See
[RootFS composition](../rootfs-composition.md).

The retained production source-query record is
v86 RootFS sources (retired capture).
The old-daemon diagnostic (retired capture)
records an unsupported-query/upgrade boundary. These are regression inputs,
not measurements or certification of the current checkout.

## Performance

Use [performance contracts](../performance.md) for controlled CPU, latency,
continuity and memory checks. A debug build, fixture trace, old binary or stored
metric cannot replace fresh source-bound release measurements.
