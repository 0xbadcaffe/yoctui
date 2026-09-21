def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/yoctui")
    parser.add_argument("--rate", type=int, default=4_000)
    parser.add_argument("--duration-seconds", type=float, default=1.0)
    parser.add_argument("--observation-seconds", type=float, default=1.5)
    parser.add_argument("--expect-pre-backpressure-failure", action="store_true")
    parser.add_argument("--include-slow-client", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if (
        args.rate < 2_000
        or args.duration_seconds < 0.1
        or args.observation_seconds <= 0
    ):
        parser.error("rate must be >=2000 and durations must be positive")
    binary = args.binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error(f"Yoctui binary is not executable: {binary}")

    begun = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="yoctui-event-flood-") as directory:
        root = Path(directory)
        daemon, socket_path, generator_report = start_daemon(
            binary, root, args.rate, args.duration_seconds
        )
        observed: set[str] = set()
        observed_pressure: dict[str, int] = {}
        frame_count = 0
        snapshots = 0
        resyncs = 0
        last_sequence: int | None = None
        ordered_sequences = True
        rss_samples: list[int] = []
        client_continuity = False
        generated: dict[str, object] | None = None
        slow_client: ProtocolClient | None = None
        client: ProtocolClient | None = None
        try:
            client = ProtocolClient(socket_path)
            initial = client.attach()
            runtime_record = json.loads(
                (socket_path.parent / "daemon.json").read_text()
            )
            if runtime_record.get("daemon_instance_id") != client.daemon_instance_id:
                raise RuntimeError(
                    "fixture daemon runtime identity differs from attachment"
                )
            readiness_started = time.monotonic()
            generation = wait_for_metadata_ready(client, initial, root / "build")
            readiness_seconds = time.monotonic() - readiness_started
            if args.include_slow_client:
                slow_client = ProtocolClient(
                    socket_path, 11, receive_buffer_bytes=4_096
                )
                slow_client.attach()
            initial_snapshot_bytes = len(
                json.dumps(initial, separators=(",", ":")).encode()
            )
            classify_snapshot(initial, observed)
            measurement_started = time.monotonic()
            build_acknowledged = False
            next_rss_sample = measurement_started
            daemon_cpu_started = process_cpu_seconds(daemon.pid)
            client.send(
                {
                    "type": "command",
                    "request_id": 1,
                    "expected_generation": generation,
                    "command": {
                        "type": "start_build",
                        "targets": ["event-flood-fixture"],
                        "task": None,
                        "force": False,
                    },
                }
            )
            report_seen_at: float | None = None
            absolute_deadline = (
                time.monotonic() + args.duration_seconds + args.observation_seconds + 10
            )
            while time.monotonic() < absolute_deadline:
                now = time.monotonic()
                if not build_acknowledged and now - measurement_started >= 10:
                    raise RuntimeError("fixture build acknowledgement timed out")
                if now >= next_rss_sample:
                    rss = process_rss(daemon.pid)
                    if rss is not None:
                        rss_samples.append(rss)
                    next_rss_sample += 1.0
                message = client.receive(0.05)
                if message is not None:
                    build_acknowledged |= observe_build_ack(message, 1)
                    frame_count += 1
                    if message.get("type") == "snapshot":
                        snapshots += 1
                    elif message.get("type") == "resync_required":
                        resyncs += 1
                    sequence = message.get("sequence")
                    if isinstance(sequence, int):
                        if last_sequence is not None and sequence <= last_sequence:
                            ordered_sequences = False
                        last_sequence = sequence
                    classify_message(message, observed)
                    observe_pressure(message, observed_pressure)
                if generator_report.exists() and report_seen_at is None:
                    report_seen_at = time.monotonic()
                if (
                    report_seen_at is not None
                    and time.monotonic() - report_seen_at >= args.observation_seconds
                ):
                    break
            measurement_elapsed = max(time.monotonic() - measurement_started, 0.000_001)
            daemon_cpu_finished = process_cpu_seconds(daemon.pid)
            wire_metrics = {
                "measurement_seconds": measurement_elapsed,
                "frames_received": client.frames_received,
                "frame_bytes_received": client.frame_bytes_received,
                "frames_per_second": client.frames_received / measurement_elapsed,
                "bytes_per_second": client.frame_bytes_received / measurement_elapsed,
                "frames_sent": client.frames_sent,
                "frame_bytes_sent": client.frame_bytes_sent,
                "initial_snapshot_json_bytes": initial_snapshot_bytes,
                "received_by_type": client.received_by_type,
                "daemon_cpu_seconds": (
                    daemon_cpu_finished - daemon_cpu_started
                    if daemon_cpu_started is not None
                    and daemon_cpu_finished is not None
                    else None
                ),
            }
            probe = ProtocolClient(socket_path, 10)
            probe_snapshot = probe.attach()
            classify_snapshot(probe_snapshot, observed)
            client_continuity = True
            probe.close()
            client.close()
            if not generator_report.exists():
                raise RuntimeError("event generator did not publish its bounded report")
            if not build_acknowledged:
                raise RuntimeError("fixture build acknowledgement was not observed")
            generated = json.loads(generator_report.read_text(encoding="utf-8"))
        finally:
            if client is not None:
                client.socket.close()
            if slow_client is not None:
                slow_client.socket.close()
            stop_daemon(daemon)
        if generated is None:
            raise RuntimeError("event generator report was not loaded")

        sent_names = {entry["name"] for entry in generated["critical_sent"]}
        required_sent = sent_names & CRITICAL_NAMES
        critical_received = sorted(required_sent & observed)
        missing = sorted(required_sent - observed)
        important_transition_missing = sorted(
            (sent_names & IMPORTANT_TRANSITION_NAMES) - observed
        )
        coalescible_missing = sorted((sent_names & COALESCIBLE_NAMES) - observed)
        retention_passed = required_sent.issubset(observed)
        known_failure = (
            "build_terminal" in sent_names
            and "build_terminal" not in observed
            and client_continuity
        )
        record = {
            "schema": SCHEMA,
            "status": "observed",
            "startup": {
                "readiness_seconds": readiness_seconds,
                "ready_generation": generation,
                "daemon_instance_id": client.daemon_instance_id,
                "build_acknowledged": build_acknowledged,
                "measurement_started_after_readiness": True,
            },
            "identity": {
                "source_base_revision": subprocess.check_output(
                    ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
                ).strip(),
                "binary_path": str(binary),
                "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            },
            "configuration": {
                "rate_events_per_second": args.rate,
                "duration_seconds": args.duration_seconds,
                "observation_seconds_after_generator_terminal": args.observation_seconds,
                "production_path": [
                    "fixture_bridge",
                    "bridge_backend",
                    "daemon_bitbake_supervisor",
                    "daemon_reducer_journal",
                    "unix_ipc",
                    "attached_protocol_client",
                ],
                "slow_client_enabled": args.include_slow_client,
            },
            "generator": generated,
            "client": {
                "frames_received": frame_count,
                "snapshot_replacements": snapshots,
                "resync_requests": resyncs,
                "event_sequences_strictly_increasing": ordered_sequences,
                "connection_continuity": client_continuity,
                "reconnect_probe_succeeded": client_continuity,
                "critical_received": critical_received,
                "critical_missing": missing,
                "important_transition_missing": important_transition_missing,
                "coalescible_missing": coalescible_missing,
                "wire_metrics": wire_metrics,
                "pressure": observed_pressure,
            },
            "bounds": {
                "daemon_rss_initial_bytes": rss_samples[0] if rss_samples else None,
                "daemon_rss_max_bytes": max(rss_samples, default=None),
                "daemon_rss_final_bytes": rss_samples[-1] if rss_samples else None,
                "journal_retained_events": 4096,
                "snapshot_build_events": 2048,
                "snapshot_recent_logs": 512,
                "supervisor_ingress": "bounded_priority_lanes",
                "supervisor_reliable_events": 512,
                "supervisor_cosmetic_events": 512,
                "per_client_backlog_events": 4_096,
                "slow_client_write_deadline_milliseconds": 5000,
                "event_write_slice_bytes": 65536,
                "pending_event_frames_per_client": 1,
            },
            "result": {
                "critical_retention_passed": retention_passed,
                "expected_pre_backpressure_terminal_starvation_observed": known_failure,
                "runtime_seconds": time.monotonic() - begun,
            },
        }
        rendered = json.dumps(record, indent=2) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(rendered, encoding="utf-8")
        print(rendered, end="")
        if not ordered_sequences or not client_continuity:
            failed = []
            if not ordered_sequences:
                failed.append("event sequence ordering")
            if not client_continuity:
                failed.append("healthy-client continuity")
            print(
                "event flood gate failed: " + ", ".join(failed),
                file=sys.stderr,
            )
            return 1
        if args.expect_pre_backpressure_failure:
            return 0 if known_failure and not retention_passed else 1
        if not retention_passed:
            print(
                "event flood gate failed: missing critical events "
                + ", ".join(missing),
                file=sys.stderr,
            )
            return 1
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
