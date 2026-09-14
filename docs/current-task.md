# Current Task

**ID:** PERF-VERIFY-001
**Title:** Measure release idle overhead and deliver startup responsiveness
**Status:** DONE

All 724 registry tasks are DONE in v0.1.107. Automatic initialized-workspace
compatibility queries, capability probes, recipe inventory and their children
run at Unix nice 10. The daemon, attached client, requested builds and explicit
interactive commands retain inherited priority. The operator guide now uses a
release build for interactive launches.

The isolated release baseline measured 0.1249% idle-daemon CPU, 0.1248%
attached-daemon CPU, 0.1248% idle-client CPU and 0.3744% combined CPU of one
logical CPU, below the 1.00% contract. During a real initialized Poky startup,
all 96 observed automatic discovery processes ran at nice 10; the attached
daemon/client measured 0.0000% combined over 20 seconds, and the first frame
arrived in 10.99 ms. No build was submitted.

Shared normal-path, bounded text, BitBake identifier, unique bounded insertion,
spawn retry and process-priority helpers now live in `yoctui-utils`. Platform
prefixes are accepted as valid absolute-path prefixes, and personal absolute
defaults were removed from the Poky helper scripts. All library roots remain
below 1000 lines.

Final verification covers 1,624 Rust tests (four existing ignored), 52 bridge
tests, strict workspace Clippy, formatting, documentation/CLI/headless/doctor,
version/layout/CI/roadmap checks and sixteen deterministic PNGs. The PNG and
cell-golden changes are version text only. This idle/startup check does not
claim a new live-build saturation certification.
