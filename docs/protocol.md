# Bridge protocol

Recipe inventory requests may opt into `chunked: true`. Only opted-in callers
receive `recipes_chunk` records: zero-based `offset`, fixed `total`, explicit
`complete`, and `recipes`. Legacy callers retain the single `recipes` response;
new clients still accept bounded legacy responses. Chunks share the exact
request correlation and monotonic envelope sequence. Completion is accepted
only with contiguous offsets and exactly the declared total, never on EOF.
Each chunk stays within 512 KiB; one transfer permits at most 16,384 recipes
and 3 MiB of compact serialized recipe data. Empty intermediate chunks, changed
totals, oversized records and aggregate overflow fail explicitly. The bridge
1 MiB line limit and daemon 4 MiB snapshot limit remain unchanged.

Each UTF-8 line is one JSON envelope: `protocol_version` (currently 1), monotonic `sequence`, optional `correlation_id`, and tagged `message`. Maximum line length is 1 MiB. Both the Python bridge and Rust transport reject oversized partial lines before processing a complete frame. Unsupported versions, malformed input, and unknown commands produce typed `command_failed` responses. Unknown incoming events deserialize safely.

Commands: `hello`, `inspect_workspace`, `start_build`, `cancel_build`, `list_recipes`, `list_layers`, `get_variable`, `shutdown`. Events: `hello_ack`, `workspace`, lifecycle/task/log events, `command_failed`, `protocol_error`, and `bridge_shutdown`. `build_completed` carries an optional `exit_code` when the backend supplies one. New optional fields are allowed; consumers must not reinterpret unknown events.

## Unresolved queue statistics

Native queue events without a resolved recipe identity emit `task_stats` with
the existing typed `stats` fields, instead of a guessed `task_queued` recipe.
Bridge consumers accept this aggregate-only event without creating a task row.
Daemon IPC requires no new event tag: the CLI publishes existing running
BitBake job progress; a current started-build checkpoint consumes those counters.
Other job kinds and terminal/absent build authority cannot replace build totals.

## Daemon job identity

Job IDs identify one daemon-owned operation across supervisor types and retained
recovery history. Non-Raw jobs share checked allocation in IDs 1 through
2^60 - 1; Raw retains its existing high-bit namespace. Recovery reserves all
retained low-namespace IDs before accepting new work. Exhaustion rejects new
work without spawning a worker or reusing an ID. No wire field or version changes
are required. Session IDs remain distinct from job IDs, and recovered history
does not restore a live process or cancellation authority.

## Daemon build counters

Daemon snapshots may include `build_progress` with nonnegative `completed` and
an optional nonzero `total`. These aggregate values are reduced from typed build
events before task-row compaction, rather than reconstructed from retained row
counts. Reset clears both counters; queued/started task statistics update their
authority; completion advances the observed count once per retained task
identity; successful build completion reconciles to a known total. Unknown totals
stay absent. Arithmetic is bounded, and duplicate completions do not advance the
aggregate unless that task was queued or started again.

The optional field is omitted when no aggregate checkpoint exists. Older
snapshots remain decodable and use their legacy event projection; a partial
legacy snapshot does not become a complete aggregate merely because a later
completion arrives. The 4 MiB frame and 2,048 retained build-event limits are
unchanged. Clients install these counters only with Current snapshot authority.

## Daemon build timing

Daemon Started and TaskStarted events optionally carry `started_unix_ms`.
TaskCompleted carries optional `started_unix_ms` and `finished_unix_ms`;
Completed carries optional `finished_unix_ms`. These unsigned Unix-millisecond
values are observations captured by the daemon CLI, not historical timestamps
inferred from filenames, logs, or a client's attachment time. Missing fields
remain absent and are omitted from serialization. Existing wire tags and
protocol versions are unchanged; older clients can ignore the added fields.

Before completion replaces the retained task start, the bounded journal copies
its timestamp into that completion. Duplicate completion events retain the
first observed end; reset discards the previous build's lifecycle rows. No
unbounded task-timing index is added, and the 2,048-event/4-MiB limits remain.
Clients use explicit observed-time reducer inputs, checked date conversion and
elapsed arithmetic. Missing or invalid evidence is unavailable; terminal
durations never use a later client clock. These fields do not persist or
restore live process ownership across daemon restart.

## Daemon compatibility snapshots

The optional daemon capability rootfs_sources advertises the read-only
inspect_rootfs_sources command. Its query binds the full 128-bit daemon
instance, compatibility generation and exact rootfs request/image identity.
The request also carries the ordinary optimistic daemon generation. A typed
rootfs_sources CommandOutcome returns those same identifiers and optional
IMAGE_MANIFEST, PKGDATA_DIR and IMAGE_ROOTFS paths (4096 bytes per path).
This result is sent only to its requesting connection, not journaled or
broadcast. Normal command replies and snapshot schemas are unchanged, and
old clients can decode the unknown optional capability without receiving the
new outcome. Clients must not send the new command to an unadvertised daemon.

One connection-owned metadata worker may run at a time; startup/active-build
conflicts reject promptly. The worker uses daemon-initialized recipe-scoped
queries through the selected authorized command or negotiated API, with
bounded handshake/query/cleanup and cancellation on
disconnect/shutdown. The client performs acquisition off its input loop,
allows at most three explicit stale-generation retries, bounds interleaved
messages, and never retries a lost reply. Returned paths remain subject to
the existing exact manifest and canonical contained filesystem scan. Current
replica/full-instance/compatibility checks gate result installation; presentation
short IDs are not authority. Missing paths stay None; cleaned reported paths
are retained so the adapter can explain their absence accurately.

Before resolving a bundled backend snapshot, the daemon may invoke the bridge
as a separate one-shot `--probe-capabilities` process. This is not a new NDJSON
or daemon IPC command. Its stdout contains only a bounded
`yoctui.bridge-capability-probe.v1` JSON object with canonical `build_directory`,
exact `bitbake_version` and unique catalog backend `capabilities` tokens.
Diagnostics go to stderr. Configuration preparation, a server ping and API
inspection are permitted; recipe parsing, builds and cancellation are not.
The parent enforces a 30-second deadline and 64-KiB per-stream output bound.
Failed, malformed, mismatched or custom-bridge probes remain inconclusive.
Only the daemon resolves this evidence; normal operation handshakes still
negotiate their actual capabilities against the resulting authority.

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
