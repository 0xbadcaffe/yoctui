# Architecture

The workspace is directional: model holds pure domain state; protocol owns stable wire types; app maps application input; bitbake owns adapters; UI renders only; CLI owns process startup. Actions enter `update`, which can request an effect. Backend work does not mutate UI state.

Logs use bounded `VecDeque` retention with byte and entry caps; dropped counts are visible, including separate warning and error eviction counts. The process backend preserves its inherited environment and captures both output streams. The bridge uses NDJSON, reports environment-derived workspace values and detected BitBake version when available, and leaves diagnostics on stderr. Terminal ownership is RAII-based; dropping the guard restores raw mode and alternate screen.
