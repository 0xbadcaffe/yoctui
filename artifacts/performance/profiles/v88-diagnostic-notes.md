# v0.1.88 real-build CPU diagnosis

Diagnostic only; this is not unprofiled release acceptance evidence. The
unchanged acceptance thresholds rejected the separately retained unprofiled
360-sample v88 capture at 1.0334662486% combined CPU.

The preserved v88 release binary
`/home/bspguy-dev/.local/state/yoctui-v88-release.TBqp2N/yoctui`, SHA-256
`be5fa7261cd3ebaf7f6f1786eabb46736419d043086d0e92224f3cc63f48b510`,
ran the committed `d2214e82974a5be708a7cc40f1532254d7c7de63` capture harness
against `/home/bspguy-dev/.local/state/yoctui-release-poky.xWaZbs` using the
same isolated offline eight-worker linux-yocto cleansstate/compile procedure.
All original Poky/OpenBMC data and normal installation were preserved.

An external controller imported the unmodified capture, intercepted its first
process sample after actual do_compile and 100 input probes, verified both
exact child executable/argument identities, then started profiling after ten
seconds. It invoked:

```sh
scripts/capture-runtime-profile.sh \
  --scenario v88-real-poky-diagnostic --duration 60 \
  --binary /home/bspguy-dev/.local/state/yoctui-v88-release.TBqp2N/yoctui \
  --revision d2214e82974a5be708a7cc40f1532254d7c7de63 \
  --pid daemon=2433848 --pid client=2457774
```

No Cargo work ran during this diagnostic. The profile contains 275 userspace
cycle samples at 499 Hz with LBR call graphs; no lost samples or unresolved
stack weight. The controller's 100-second limit expired during flamegraph
conversion after the complete 60-second recording. Conversion finished later;
the existing summarizer was then run separately against the exact perf log,
flat report, filter report, SVG and pre-recorded process identities and passed
its unchanged 50-sample/5000-ppm quality gates. All diagnostic processes exited.
The capture itself completed its 120-second observation and owned-job
cancellation/cleanup before the controller reported the conversion timeout.

The [profile summary](v88-real-poky-diagnostic.json),
[flat report](v88-real-poky-diagnostic.perf.txt) and
[flamegraph](v88-real-poky-diagnostic.svg) preserve the result.
The [workload record](v88-profiled-workload.json) retains actual configuration,
compile trigger, process identities and samples with an explicit diagnostic
role and contamination warning; it must never replace canonical real-Poky
acceptance evidence. Original unmodified output and controller remain in
`/tmp/yoctui-v88-profiled-real-poky.json` and
`/tmp/yoctui-v88-profile-controller.py`. Raw perf data remains in
`target/performance-profiles/v88-real-poky-diagnostic.0RHdmH/`.

Measured self-cycle weight: JSON string escaping 19.23%, all in the daemon
when filtering the raw report to PID 2433848. Inclusive flamegraph weight:
journal publish 43.81%, full `DaemonSnapshot` serialization 26.90%, snapshot
drop 6.54%. These are sampled cycle weights, not process CPU percentages.
Source inspection explains avoidable work: once-per-second Telemetry currently
takes the transactional full-clone/full-serialization branch even though its
snapshot reduction only increments sequence/generation. The existing proven
conservative size ledger can also cover this non-failing event reduction;
hard frame/snapshot limits, exact remeasurement when headroom is exhausted,
ordered replay and counter-overflow checks must remain unchanged.
