# Current Task

**ID:** M72-IPC-001
**Title:** Preserve client frames under temporary backpressure and terminal ownership
**Status:** IN_PROGRESS

User-requested live OpenBMC corrections supersede the blocked task below.
Dependencies: none. Files: protocol daemon_ipc, CLI main/client runtime.
Done: bounded resumable event writes, terminal-safe diagnostics, real Unix
socket regression tests. Update UI/architecture, registry, status and current
task, commit, then continue M72-CACHE-001 and M72-LAYOUT-001. Preserve the
active user daemon/build.

Verification:
```bash
cargo test -p yoctui-protocol daemon_ipc
cargo test -p yoctui interactive_daemon
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Retained external blocker: M67-LIVE-EVIDENCE-001

CI-RELEASE-GATES-001 is DONE: GitHub Actions compatibility and release-quality
gates now recognize the restored README compatibility rule and compact
80-column terminal frames. The deterministic compatibility, performance,
formatting, Clippy and version checks pass locally.

The remaining task is blocked on a genuine current-source real-Poky
performance capture, documented below.

README-DBUS-UDEV-001 is DONE: system D-Bus and udev rules screenshots are
in the README image-inspection gallery. All 23 screenshots and the baseline/
documentation checks pass.

README-SYSTEMD-001 is DONE: the README includes a production-rendered offline
systemd Services screenshot beside rootfs composition. All 21 screenshots and
the baseline/documentation checks pass.

README-REAL-DTS-001 is DONE: the Device Tree editor screenshot uses the Linux
v6.6 NXP i.MX8MP EVK source, with attributed MIT licensing and verified syntax
colors. Baseline, screenshot and documentation checks pass.

README-ONBOARDING-001 is DONE: the README now follows setup and daily Yocto
workflows, retains all 20 screenshots and technical information, and uses direct
operator wording. Documentation, screenshot and baseline checks pass.

All requested M67/M68/M69 implementation and publication tasks are DONE. The source and UI fixes,
GitUI integration, offline workbench, durable saved history and updated 20-screen
README gallery are committed for master. Version 0.1.118 is published on crates.io
for all seven public crates.
Full workspace tests, strict Clippy, formatting, 52 bridge tests, screenshot
provenance checks and the native GitUI PTY smoke pass. No release-performance
certification is claimed from fixtures or fake-process timing.

External prerequisite: a genuine current-source/binary-bound real-Poky
performance capture for the documented Yocto 6.0.2 linux-yocto workload.
The retained `artifacts/performance/real-poky/manifest.json` is bound to source
base `d2214e82974a5be708a7cc40f1532254d7c7de63`; 52 recorded source hashes now
differ, including changes predating this request. Historical evidence stays intact.

Reproduce the independent prerequisite check:

```bash
python3 - <<'PYCODE'
from pathlib import Path
import hashlib, json
m = json.loads(Path("artifacts/performance/real-poky/manifest.json").read_text())
stale = [name for name, expected in m["sources"].items()
         if hashlib.sha256(Path(name).read_bytes()).hexdigest() != expected]
print("Source digest mismatches:", len(stale))
raise SystemExit(bool(stale))
PYCODE
```

The completion gate reports this required task as BLOCKED. The broader
performance verifier was stopped after its static contracts passed because the
independent mandatory evidence check already proves the capture is stale.
Follow the existing capture procedure in `docs/performance.md`, then verify:

```bash
./scripts/verify-performance.sh --real-poky-evidence
./scripts/verify-completion.sh
```

After M72, this external evidence prerequisite remains required.
