def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--warmup-seconds", type=float, default=1.0)
    parser.add_argument("--observations", type=int, default=100)
    args = parser.parse_args()
    if len(args.revision) != 40:
        parser.error("revision must be a full 40-character Git commit")
    if args.warmup_seconds < 0.25:
        parser.error("warmup must be at least 0.25 seconds")
    if args.observations < 100 or args.observations > 1_000:
        parser.error("observations must be within 100..1000")
    binary = args.binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error("Yoctui binary is not executable")

    fixture = Path(tempfile.mkdtemp(prefix="yoctui-ipc-latency-"))
    saturation_path = fixture / "saturation.json"
    saturation_events = fixture / "saturation.jsonl"
    daemon: subprocess.Popen[str] | None = None
    saturation: subprocess.Popen[str] | None = None
    client: ProtocolClient | None = None
    observation = None
    command_samples: list[dict[str, int | float]] = []
    cancellation_samples: list[dict[str, int | float]] = []
    reconnect_succeeded = False
    detach_acknowledged = False
    detach_latency_ms: float | None = None
    saturation_alive_for_all_samples = True
    try:
        daemon, socket_path = start_daemon(binary, fixture)
        client = ProtocolClient(socket_path, 31)
        snapshot = client.attach()
        generation = snapshot.get("generation")
        if not isinstance(generation, int):
            raise RuntimeError("daemon snapshot omitted generation")
        observation = Observation(
            client, args.observations, EVENT_WARMUP_OBSERVATIONS
        )
        saturation = subprocess.Popen(
            [
                str(ROOT / "scripts/cpu-saturation-harness.py"),
                "--warmup-seconds",
                str(args.warmup_seconds),
                "--duration-seconds",
                "3",
                "--minimum-worker-cpu-percent",
                "25",
                "--event-log",
                str(saturation_events),
                "--output",
                str(saturation_path),
            ],
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
        )
        wait_saturation_ready(saturation, saturation_events)
        time.sleep(args.warmup_seconds + 0.1)

        start_result, _, _ = observation.command(
            1,
            None,
            {
                "type": "start_build",
                "targets": ["ipc-latency-fixture"],
                "task": None,
                "force": False,
            },
        )
        if not accepted(start_result):
            raise RuntimeError(f"fixture build was not accepted: {start_result}")
        job = wait_for_job(observation, None, {"connecting", "running"})
        job_id = job.get("id")
        if not isinstance(job_id, int):
            raise RuntimeError("build job event omitted its numeric identity")

        while len(observation.event_samples) < args.observations:
            observation.receive()
            saturation_alive_for_all_samples &= saturation.poll() is None

        for index in range(1, args.observations + 1):
            request_id = 1_000 + index
            result, sent_ns, received_ns = observation.command(
                request_id,
                None,
                {"type": "cancel_job", "job_id": 18_446_744_073_709_551_615},
            )
            outcome, rejection_code = outcome_identity(result)
            if outcome != "rejected" or rejection_code != "not_found":
                raise RuntimeError(f"command receipt probe returned an unexpected result: {result}")
            command_samples.append(
                {
                    "sequence": index,
                    "request_id": request_id,
                    "sent_ns": sent_ns,
                    "acknowledged_ns": received_ns,
                    "latency_ms": (received_ns - sent_ns) / 1_000_000,
                }
            )
            saturation_alive_for_all_samples &= saturation.poll() is None

        accepted_cancellations = 0
        for batch_start in range(1, args.observations + 1, 50):
            batch_end = min(batch_start + 49, args.observations)
            cancellation_sent: dict[int, tuple[int, int]] = {}
            batch_accepted = 0
            for index in range(batch_start, batch_end + 1):
                request_id = 2_000 + index
                sent_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
                client.send(
                    {
                        "type": "command",
                        "request_id": request_id,
                        "expected_generation": None,
                        "command": {"type": "cancel_job", "job_id": job_id},
                    }
                )
                cancellation_sent[request_id] = (index, sent_ns)
            cancellation_deadline = time.monotonic() + 5.0
            while cancellation_sent and time.monotonic() < cancellation_deadline:
                received = client.receive(
                    max(0.001, cancellation_deadline - time.monotonic())
                )
                if received is None:
                    break
                message, received_ns = received
                observation.process(message, received_ns)
                request_id = message.get("request_id")
                if (
                    message.get("type") != "command_result"
                    or request_id not in cancellation_sent
                ):
                    continue
                outcome, rejection_code = outcome_identity(message)
                if outcome == "accepted":
                    batch_accepted += 1
                    accepted_cancellations += 1
                elif outcome != "rejected" or rejection_code != "not_found":
                    raise RuntimeError(f"unexpected cancellation outcome: {message}")
                index, sent_ns = cancellation_sent.pop(int(request_id))
                cancellation_samples.append(
                    {
                        "sequence": index,
                        "request_id": request_id,
                        "job_id": job_id,
                        "sent_ns": sent_ns,
                        "acknowledged_ns": received_ns,
                        "latency_ms": (received_ns - sent_ns) / 1_000_000,
                        "outcome": outcome,
                        "rejection_code": rejection_code,
                    }
                )
                saturation_alive_for_all_samples &= saturation.poll() is None
            if cancellation_sent:
                raise RuntimeError(
                    f"missing {len(cancellation_sent)} cancellation acknowledgements"
                )
            if batch_accepted == 0:
                raise RuntimeError("cancellation batch did not cancel its active build")
            wait_for_job(observation, job_id, {"failed", "exited", "lost"})
            if batch_end == args.observations:
                continue
            next_result, _, _ = observation.command(
                3_000 + batch_end,
                None,
                {
                    "type": "start_build",
                    "targets": ["ipc-latency-fixture"],
                    "task": None,
                    "force": False,
                },
            )
            if not accepted(next_result):
                raise RuntimeError(f"replacement fixture build was not accepted: {next_result}")
            job = wait_for_job(observation, None, {"connecting", "running"})
            job_id = job.get("id")
            if not isinstance(job_id, int):
                raise RuntimeError("replacement build omitted numeric job identity")
        cancellation_samples.sort(key=lambda sample: int(sample["sequence"]))

        detach_sent_ns, detach_received_ns = client.detach()
        detach_acknowledged = True
        detach_latency_ms = (detach_received_ns - detach_sent_ns) / 1_000_000
        client = None
        saturation_alive_for_all_samples &= saturation.poll() is None

        probe = ProtocolClient(socket_path, 32)
        probe.attach()
        probe.close()
        reconnect_succeeded = True
        saturation_alive_for_all_samples &= saturation.poll() is None

        saturation_stderr = saturation.communicate(timeout=10.0)[1]
        if saturation.returncode != 0:
            raise RuntimeError(
                f"saturation fixture failed after measurement: {saturation_stderr.strip()}"
            )
        load = json.loads(saturation_path.read_text(encoding="utf-8"))
        event_latencies = [float(sample["latency_ms"]) for sample in observation.event_samples]
        command_latencies = [float(sample["latency_ms"]) for sample in command_samples]
        cancellation_latencies = [
            float(sample["latency_ms"]) for sample in cancellation_samples
        ]
        ordered = all(
            current > previous
            for previous, current in zip(
                observation.protocol_sequences, observation.protocol_sequences[1:]
            )
        )
        record = {
            "schema": SCHEMA,
            "revision": args.revision,
            "captured_at_unix_ms": int(time.time() * 1_000),
            "binary": {
                "path": str(binary),
                "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            },
            "host": {
                "logical_cpus": os.cpu_count(),
                "affinity_cpus": sorted(os.sched_getaffinity(0)),
                "kernel": os.uname().release,
            },
            "configuration": {
                "clock": "CLOCK_MONOTONIC",
                "transport": "AF_UNIX SOCK_STREAM length-prefixed JSON",
                "warmup_seconds": args.warmup_seconds,
                "observations_per_path": args.observations,
                "event_warmup_observations": EVENT_WARMUP_OBSERVATIONS,
                "event_rate_per_second": 500,
                "load": "one pinned worker per affinity CPU; no deliberately free CPU",
                "event_path": [
                    "fixture_bridge",
                    "bridge_backend",
                    "daemon_bitbake_supervisor",
                    "daemon_snapshot_journal",
                    "unix_ipc",
                    "attached_protocol_client",
                ],
                "command_method": "client cancel_job for a deliberately absent u64 job identity through daemon dispatch to correlated not_found command_result receipt; measured round trip is a conservative upper bound on daemon receipt without command workload",
                "cancellation_method": "two protocol-compliant batches of 50 pipelined cancel_job requests against active BitBake jobs; every request receives an ordered correlated command_result and each batch proves at least one accepted live-supervisor cancellation",
            },
            "summary": {
                "daemon_event_to_client_ms": summarize(event_latencies),
                "client_command_to_daemon_ms": summarize(command_latencies),
                "cancellation_request_to_ack_ms": summarize(cancellation_latencies),
                "accepted_cancellation_requests": accepted_cancellations,
            },
            "samples": {
                "daemon_event_to_client": observation.event_samples,
                "client_command_to_daemon": command_samples,
                "cancellation_request_to_ack": cancellation_samples,
            },
            "continuity": {
                "initial_generation": generation,
                "primary_client_connected_until_explicit_detach": True,
                "detach_acknowledged": detach_acknowledged,
                "detach_latency_ms": detach_latency_ms,
                "detach_under_saturation": saturation_alive_for_all_samples,
                "reconnect_succeeded": reconnect_succeeded,
                "reconnect_under_saturation": saturation_alive_for_all_samples,
                "backend_disconnect_events": observation.backend_disconnects,
                "protocol_sequences_strictly_increasing": ordered,
                "protocol_event_count": len(observation.protocol_sequences),
            },
            "saturation": {
                "alive_for_every_observation": saturation_alive_for_all_samples,
                "completed_after_measurement": load["status"] == "completed",
                "configuration": load["configuration"],
                "achieved": load["achieved"],
                "cleanup": load["cleanup"],
            },
        }
        rendered = json.dumps(record, indent=2, sort_keys=True) + "\n"
        args.output.parent.mkdir(parents=True, exist_ok=True)
        temporary = args.output.with_suffix(args.output.suffix + ".tmp")
        temporary.write_text(rendered, encoding="utf-8")
        temporary.replace(args.output)
        print(rendered, end="")
        return 0
    finally:
        if client is not None:
            client.close()
        if saturation is not None and saturation.poll() is None:
            stop_process(saturation)
        if daemon is not None:
            stop_process(daemon)
        shutil.rmtree(fixture)


if __name__ == "__main__":
    raise SystemExit(main())
