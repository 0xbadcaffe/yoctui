# Current Task

**ID:** M67-LIVE-EVIDENCE-001
**Title:** Supply current-source real-Poky release performance evidence
**Status:** BLOCKED

The four requested implementation/documentation tasks are DONE and committed.
Their full workspace, strict Clippy, formatting, 52 bridge tests and roadmap
checks pass. The controlled startup probe improved readiness from 3.246 s to
116 ms; this is fake-process regression evidence, not live-Yocto certification.

External prerequisite: a genuine current-source/binary-bound real-Poky
performance capture for the documented Yocto 6.0.2 linux-yocto workload.
The retained manifest at `artifacts/performance/real-poky/manifest.json` is bound
to source base `d2214e82974a5be708a7cc40f1532254d7c7de63`; 49 recorded source
hashes differ, including older changes predating this request.

Reproduce the prerequisite check:

```bash
python3 - <<'PYCODE'
from pathlib import Path
import hashlib, json
manifest = json.loads(Path("artifacts/performance/real-poky/manifest.json").read_text())
stale = [name for name, expected in manifest["sources"].items()
         if hashlib.sha256(Path(name).read_bytes()).hexdigest() != expected]
print("Source digest mismatches:", len(stale))
for name in stale:
    print(name)
raise SystemExit(bool(stale))
PYCODE
```

The completion gate was started and passed compatibility and the full workspace
suite again. Remaining checks were stopped after this independent mandatory
prerequisite check proved the retained live evidence cannot certify current
sources. No completion-gate PASS is claimed. Historical manifests, checksums and
thresholds remain unchanged. Follow the existing capture procedure in
`docs/performance.md`, then verify:

```bash
./scripts/verify-performance.sh --real-poky-evidence
./scripts/verify-completion.sh
```

No other eligible incomplete task remains.
