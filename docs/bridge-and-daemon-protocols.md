# Bridge and daemon protocols

See [Architecture](architecture.md) for ownership and [source types](../crates/yoctui-protocol/src/lib.rs)
for exact schemas. These are separate transports.

## Bridge framing

Each UTF-8 NDJSON envelope has protocol_version=1, monotonic sequence, optional
correlation_id, and tagged message. Lines/partial lines are capped at 1 MiB.
Malformed/unsupported requests produce typed failures; unknown events cannot enable actions.

Core commands: hello, inspect_workspace, start_build, cancel_build, list_recipes,
list_layers, get_variable, shutdown. Events include workspace, lifecycle/task/log,
hello_ack, command_failed, protocol_error, bridge_shutdown. build_completed may include exit_code.

Opt-in chunked recipe transfers use recipes_chunk with contiguous zero-based offset,
fixed total, complete, and matching correlation/sequence. Limits: 512 KiB/chunk,
16,384 recipes, 3 MiB aggregate. EOF/empty intermediate chunks/changed totals/overflow
fail. Non-opted callers receive the bounded legacy recipes response.

## Daemon snapshots and jobs

Protocol 1.5 is negotiated exactly before commands. update_recipe_patch accepts
one recipe and absolute configured-layer destination, producing reviewed
`devtool update-recipe --mode patch --append` argv. Snapshots are capped at 4 MiB,
build retention at 2,048 events. Non-Raw job IDs use 1..2^60-1; Raw has a separate
high-bit namespace. Recovery reserves IDs without restoring process/cancellation ownership.

Unresolved native queue events report task_stats, not guessed recipe rows.
Optional build_progress carries nonnegative completed and optional nonzero total,
reduced before compaction. Unknown totals stay absent; duplicates advance once unless
requeued/restarted. Successful completion reconciles known totals. Legacy partial
snapshots cannot acquire aggregate authority from an unrelated completion.

Optional started_unix_ms/finished_unix_ms are daemon observations. Completion retains
the original start and first end; reset clears prior lifecycle. Missing/invalid times
stay absent; terminal duration cannot use a later client clock.

## Rootfs requests

Advertised rootfs_sources capability enables inspect_rootfs_sources. Query/outcome
bind full 128-bit instance, compatibility generation, optimistic daemon generation,
and exact image/request. IMAGE_MANIFEST/PKGDATA_DIR/IMAGE_ROOTFS paths are optional,
at most 4096 bytes each. Replies go only to the requester, without journal/broadcast.

One connection-owned metadata worker runs at a time; busy/startup/build conflicts
reject. Queries/cleanup are bounded and cancelled on disconnect/shutdown. Clients
allow at most three explicit stale retries, never retry a lost reply, and bound
interleaved frames. Installation validates full authority; scans validate containment.
Missing paths remain None; reported cleaned paths remain available for diagnosis.

## Compatibility snapshots

Optional schema-v1 snapshots contain exact environment, nonzero generation,
unique capability IDs, five states, bounded reasons/evidence, and available-route
implementations. compatibility_changed replaces the complete snapshot with increasing
inner generation and matching daemon event authority. Invalid identity/paths, duplicate
IDs, oversized collections/argv/text, contradictions, or stale versions are rejected.
Future enums decode Unknown; missing snapshots cannot imply availability.

The separate one-shot --probe-capabilities bridge mode returns only bounded
`yoctui.bridge-capability-probe.v1` JSON: canonical build_directory, exact
bitbake_version, unique catalog capabilities. Diagnostics use stderr. Deadline 30 s,
64 KiB per stream; no recipe parsing/build/cancellation. Failed/custom/mismatched probes
are inconclusive; ordinary operation handshakes still negotiate actual capabilities.
