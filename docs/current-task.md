# Current Task

**ID:** M67-LIVE-EVIDENCE-001
**Title:** Supply current-source real-Poky release performance evidence
**Status:** BLOCKED

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

No other eligible incomplete task remains.
