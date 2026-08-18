# Bridge protocol

Each UTF-8 line is one JSON envelope: `protocol_version` (currently 1), monotonic `sequence`, optional `correlation_id`, and tagged `message`. Maximum line length is 1 MiB. Both the Python bridge and Rust transport reject oversized partial lines before processing a complete frame. Unsupported versions, malformed input, and unknown commands produce typed `command_failed` responses. Unknown incoming events deserialize safely.

Commands: `hello`, `inspect_workspace`, `start_build`, `cancel_build`, `list_recipes`, `list_layers`, `get_variable`, `shutdown`. Events: `hello_ack`, `workspace`, lifecycle/task/log events, `command_failed`, `protocol_error`, and `bridge_shutdown`. `build_completed` carries an optional `exit_code` when the backend supplies one. New optional fields are allowed; consumers must not reinterpret unknown events.

## Daemon compatibility snapshots

The persistent daemon snapshot optionally carries compatibility schema v1. It
contains the authoritative environment identity, a non-zero snapshot
generation, and unique stable capability IDs. Each capability transmits one of
the five product states, its bounded reason and evidence, and the selected
implementation only when the state is available. `compatibility_changed`
events replace the complete compatibility snapshot; they are correlated both
by the daemon event sequence/generation and by a strictly increasing inner
compatibility generation.

Receivers validate the complete replacement before applying it. Invalid paths,
identity authorities, duplicate IDs, oversized text/collections/argv,
unsupported schema versions, contradictory evidence, and stale generations
are rejected. Unknown future state/evidence enum values decode to explicit
unknown values and never enable an action. The optional snapshot field keeps
older persisted daemon snapshots readable while absence means compatibility is
not yet known, never that all binary-known features are available.
