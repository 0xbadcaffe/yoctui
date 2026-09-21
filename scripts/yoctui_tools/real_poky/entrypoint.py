def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--build-dir", type=Path, required=True)
    parser.add_argument("--poky-root", type=Path, required=True)
    parser.add_argument("--target", default="core-image-sato")
    parser.add_argument("--task")
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--preclean", action="store_true")
    parser.add_argument("--seconds", type=int, default=180)
    parser.add_argument("--warmup-seconds", type=int, default=10)
    parser.add_argument(
        "--wait-for-task",
        help="begin warmup only after TARGET:TASK starts (for example linux-yocto:do_compile)",
    )
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.seconds < 120:
        parser.error("real Poky evidence requires at least 120 seconds")
    if args.warmup_seconds < 10:
        parser.error("real Poky evidence requires at least 10 seconds of warmup")
    binary = args.binary.resolve(strict=True)
    build_dir = args.build_dir.resolve(strict=True)
    poky_root = args.poky_root.resolve(strict=True)
    if not (build_dir / "conf/local.conf").is_file():
        parser.error("build directory is not initialized")
    repository_paths = {
        "bitbake": poky_root / "layers/bitbake",
        "openembedded_core": poky_root / "layers/openembedded-core",
        "meta_yocto": poky_root / "layers/meta-yocto",
    }
    if not all((path / ".git").exists() for path in repository_paths.values()):
        parser.error("Poky root does not contain the expected source repositories")
    if not os.environ.get("BUILDDIR") or Path(os.environ["BUILDDIR"]).resolve() != build_dir:
        parser.error("run from the initialized build environment for this build directory")

    fixture = Path(tempfile.mkdtemp(prefix="yoctui-real-poky-"))
    environment = os.environ.copy()
    for name in ("runtime", "state", "config"):
        (fixture / name).mkdir(mode=0o700)
    environment.update({
        "XDG_RUNTIME_DIR": str(fixture / "runtime"),
        "XDG_STATE_HOME": str(fixture / "state"),
        "XDG_CONFIG_HOME": str(fixture / "config"),
        "TERM": "xterm-256color",
    })
    environment.pop("YOCTUI_BRIDGE_PATH", None)
    clock_ticks = os.sysconf("SC_CLK_TCK")
    capture_start_ticks = int(float(Path("/proc/uptime").read_text().split()[0]) * clock_ticks)
    daemon = subprocess.Popen(
        [str(binary), "daemon", "foreground"], cwd=ROOT, env=environment,
        stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
    )
    client = None
    observer = None
    master = None
    drain_stop = threading.Event()
    output_clock = [0]
    samples: list[dict[str, object]] = []
    key_latencies: list[float] = []
    counts = {"events": 0, "logs": 0, "task_started": 0, "task_completed": 0, "disconnects": 0}
    event_types: dict[str, int] = {}
    pressure: dict[str, int] = {}
    cancellation = {"requested": False, "acknowledged": False, "accepted": False}
    ipc_latencies: dict[str, float] = {}
    reconnect = False
    job_id: int | None = None
    try:
        socket_path = fixture / "runtime/yoctui/daemon.sock"
        deadline = time.monotonic() + 30
        while not socket_path.exists() and daemon.poll() is None and time.monotonic() < deadline:
            time.sleep(0.02)
        if not socket_path.exists():
            raise RuntimeError("real-Poky daemon did not start")
        runtime_record_path = fixture / "runtime/yoctui/daemon.json"
        ready_deadline = time.monotonic() + 300
        while (
            not runtime_record_path.exists()
            and daemon.poll() is None
            and time.monotonic() < ready_deadline
        ):
            time.sleep(0.05)
        if not runtime_record_path.exists():
            raise RuntimeError("real-Poky daemon did not publish its ready runtime record")
        daemon_runtime_record = json.loads(runtime_record_path.read_text())
        attach_deadline = time.monotonic() + 300
        snapshot = None
        while snapshot is None and time.monotonic() < attach_deadline:
            observer = IPC.ProtocolClient(socket_path, 71)
            try:
                snapshot = observer.attach()
            except (RuntimeError, TimeoutError, OSError):
                observer.socket.close()
                observer = None
                if daemon.poll() is not None:
                    raise RuntimeError("real-Poky daemon exited during compatibility startup")
                time.sleep(0.25)
        if snapshot is None:
            raise RuntimeError("real-Poky daemon did not complete compatibility startup")
        generation = snapshot.get("generation")
        if not isinstance(generation, int):
            raise RuntimeError("daemon snapshot omitted generation")
        if snapshot.get("daemon_instance_id") != daemon_runtime_record["daemon_instance_id"]:
            raise RuntimeError("real-Poky initial snapshot belongs to another daemon instance")
        workspace_variables = wait_for_initial_workspace(observer, snapshot)
        parallelism = {
            key: workspace_variables.get(key)
            for key in ("BB_NUMBER_THREADS", "PARALLEL_MAKE")
        }
        if any(value is None for value in parallelism.values()):
            raise RuntimeError("daemon workspace omitted BitBake parallelism variables")
        observation = IPC.Observation(observer, 100, 0)

        request_id = 1
        if args.preclean:
            clean_result, _, _ = observation.command(request_id, None, {
                "type": "start_build",
                "targets": [args.target],
                "task": "cleansstate",
                "force": False,
            })
            if not IPC.accepted(clean_result):
                raise RuntimeError(f"real Poky preclean was rejected: {clean_result}")
            clean_job = IPC.wait_for_job(observation, None, {"connecting", "running"})
            clean_job_id = clean_job.get("id")
            if not isinstance(clean_job_id, int):
                raise RuntimeError("preclean job omitted job identity")
            clean_terminal = wait_for_terminal_job(observation, clean_job_id)
            if clean_terminal.get("lifecycle") != "exited":
                raise RuntimeError(f"real Poky preclean failed: {clean_terminal}")
            request_id += 1

        result, command_sent_ns, command_received_ns = observation.command(request_id, None, {
            "type": "start_build", "targets": [args.target], "task": args.task, "force": args.force,
        })
        ipc_latencies["build_command_to_ack_ms"] = (
            command_received_ns - command_sent_ns
        ) / 1_000_000
        if not IPC.accepted(result):
            raise RuntimeError(f"real Poky build was rejected: {result}")
        job = IPC.wait_for_job(observation, None, {"connecting", "running"})
        job_id = job.get("id")
        if not isinstance(job_id, int):
            raise RuntimeError("real build omitted job identity")

        workload_trigger = None
        if args.wait_for_task:
            try:
                trigger_recipe, trigger_task = args.wait_for_task.split(":", 1)
            except ValueError as error:
                raise RuntimeError("--wait-for-task must use TARGET:TASK syntax") from error
            workload_trigger = wait_for_task_started(
                observation, job_id, trigger_recipe, trigger_task
            )

        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 160, 0, 0))
        render_metrics_path = fixture / "client-render-metrics.json"
        client_environment = environment.copy()
        client_environment["YOCTUI_PERFORMANCE_METRICS_PATH"] = str(render_metrics_path)
        client = subprocess.Popen(
            [str(binary), "attach"], cwd=ROOT, env=client_environment,
            stdin=slave, stdout=slave, stderr=slave, start_new_session=True, close_fds=True,
        )
        os.close(slave)
        os.set_blocking(master, False)

        def drain() -> None:
            while not drain_stop.is_set():
                ready, _, _ = select.select([master], [], [], 0.25)
                if ready:
                    try:
                        if os.read(master, 65536):
                            output_clock[0] = time.monotonic_ns()
                    except OSError:
                        return

        thread = threading.Thread(target=drain, daemon=True)
        thread.start()
        deadline = time.monotonic() + 10
        while output_clock[0] == 0 and client.poll() is None and time.monotonic() < deadline:
            time.sleep(0.01)
        if output_clock[0] == 0:
            raise RuntimeError("interactive client did not render")

        for index in range(100):
            before = output_clock[0]
            sent = time.monotonic_ns()
            os.write(master, b"\x1b[B" if index % 2 == 0 else b"\x1b[A")
            limit = time.monotonic() + 1
            while output_clock[0] <= before and time.monotonic() < limit:
                time.sleep(0.001)
            if output_clock[0] <= before:
                raise RuntimeError(f"interactive client did not render input probe {index + 1}")
            key_latencies.append((output_clock[0] - sent) / 1_000_000)

        previous = {"daemon": proc(daemon.pid), "client": proc(client.pid)}
        previous_host = host_cpu()
        previous_bitbake = bitbake_processes(capture_start_ticks)
        previous_at = time.monotonic()
        warmup_started = previous_at
        started = warmup_started + args.warmup_seconds
        next_sample = started + 1
        measurement_initialized = False
        while time.monotonic() - started < args.seconds:
            received = observer.receive(0.05)
            message = received[0] if received is not None else None
            if received is not None:
                observation.process(received[0], received[1])
            measuring = time.monotonic() >= started
            if measuring and message is not None and message.get("type") == "event":
                counts["events"] += 1
                event = message.get("event", {})
                event_type = str(event.get("type", "unknown"))
                event_types[event_type] = event_types.get(event_type, 0) + 1
                if event.get("type") == "log": counts["logs"] += 1
                if event.get("type") == "build":
                    data = event.get("data", {})
                    kind = data.get("type")
                    if kind == "task_started": counts["task_started"] += 1
                    elif kind == "task_completed": counts["task_completed"] += 1
                    elif kind == "disconnected": counts["disconnects"] += 1
                if event.get("type") == "telemetry":
                    for key, value in event.get("data", {}).get("pressure", {}).items():
                        if isinstance(value, int): pressure[key] = max(pressure.get(key, 0), value)
            now = time.monotonic()
            if measuring and not measurement_initialized:
                previous = {"daemon": proc(daemon.pid), "client": proc(client.pid)}
                previous_host = host_cpu()
                previous_bitbake = bitbake_processes(capture_start_ticks)
                previous_at = now
                next_sample = now + 1
                next_key = now + 1
                measurement_initialized = True
                continue
            if measuring and now >= next_sample:
                current = {"daemon": proc(daemon.pid), "client": proc(client.pid)}
                current_host = host_cpu()
                current_bitbake = bitbake_processes(capture_start_ticks)
                elapsed = now - previous_at
                host_total_delta = current_host[0] - previous_host[0]
                bitbake_ticks = sum(
                    max(0, state["ticks"] - previous_bitbake.get(pid, state)["ticks"])
                    for pid, state in current_bitbake.items()
                )
                samples.append({
                    "elapsed_seconds": now - started,
                    "processes": {
                        role: {
                            "cpu_percent_one_logical_cpu": (current[role]["ticks"] - previous[role]["ticks"]) / clock_ticks / elapsed * 100,
                            "rss_bytes": current[role]["rss_bytes"],
                            "threads": current[role]["threads"],
                        } for role in current
                    },
                    "host_cpu_percent_total_capacity": (
                        (current_host[1] - previous_host[1]) / host_total_delta * 100
                        if host_total_delta > 0 else 0.0
                    ),
                    "bitbake": {
                        "cpu_percent_one_logical_cpu": bitbake_ticks / clock_ticks / elapsed * 100,
                        "processes": sorted(current_bitbake),
                    },
                })
                previous, previous_at = current, now
                previous_host = current_host
                previous_bitbake = current_bitbake
                next_sample += 1

        final_job = observation.job_states.get(job_id, job)
        if final_job.get("lifecycle") not in {"connecting", "running", "stopping"}:
            raise RuntimeError(
                f"real Poky workload was not active for the complete measurement window: {final_job}"
            )
        cancellation["requested"] = True
        result, cancel_sent_ns, cancel_received_ns = observation.command(
            9000, None, {"type": "cancel_job", "job_id": job_id}
        )
        ipc_latencies["cancellation_command_to_ack_ms"] = (
            cancel_received_ns - cancel_sent_ns
        ) / 1_000_000
        cancellation["acknowledged"] = True
        cancellation["accepted"] = IPC.accepted(result)
        observer.close()
        reconnect_started_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
        observer = IPC.ProtocolClient(socket_path, 72)
        observer.attach()
        ipc_latencies["fresh_attach_ms"] = (
            time.clock_gettime_ns(time.CLOCK_MONOTONIC) - reconnect_started_ns
        ) / 1_000_000
        reconnect = True
        process_identities = {
            "daemon": process_identity(daemon.pid),
            "client": process_identity(client.pid),
        }
        client.send_signal(signal.SIGTERM)
        client.wait(timeout=15)
        render_metrics = json.loads(render_metrics_path.read_text())
        record = {
            "schema": "yoctui.performance.real-poky.v1",
            "captured_at_utc": datetime.now(timezone.utc).isoformat(),
            "source_base_revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
            "evidence_role": "real_poky_build",
            "host": host_identity(build_dir),
            "binary": {"path": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "version": subprocess.check_output([str(binary), "--version"], text=True).strip()},
            "processes": process_identities,
            "daemon_runtime": daemon_runtime_record,
            "poky": {"release": "6.0.2", "root": str(poky_root), "build_directory": str(build_dir), "target": args.target, "task": args.task, "force": args.force, "preparation": ({"target": args.target, "task": "cleansstate", "through_daemon": True} if args.preclean else None), "machine": "qemux86-64", "distro": "poky", "parallelism": parallelism, "repositories": {name: git_identity(path) for name, path in repository_paths.items()}},
            "measurement": {"clock": "CLOCK_MONOTONIC", "warmup_seconds": args.warmup_seconds, "window_seconds": args.seconds, "sustained_active_seconds": args.seconds, "sample_count": len(samples), "statistic": "10_percent_trimmed_mean", "workload_trigger": workload_trigger, "terminal": {"columns": 160, "rows": 50}},
            "events": {**counts, "by_type": event_types, "events_per_second": counts["events"] / args.seconds},
            "pressure": pressure,
            "responsiveness": {"key_to_visible_frame_samples_ms": key_latencies, "key_to_visible_frame_p95_ms": percentile(key_latencies, 0.95), "ipc_ms": ipc_latencies},
            "rendering": render_metrics,
            "continuity": {"backend_disconnects": counts["disconnects"], "client_reconnected": reconnect, "cancellation": cancellation},
            "summary": {
                role: {
                    "cpu_trimmed_mean_percent_one_logical_cpu": trimmed_mean([s["processes"][role]["cpu_percent_one_logical_cpu"] for s in samples]),
                    "rss_max_bytes": max(s["processes"][role]["rss_bytes"] for s in samples),
                    "threads_max": max(s["processes"][role]["threads"] for s in samples),
                } for role in ("daemon", "client")
            },
            "samples": samples,
        }
        record["summary"]["combined_cpu_trimmed_mean_percent_one_logical_cpu"] = trimmed_mean([
            s["processes"]["daemon"]["cpu_percent_one_logical_cpu"]
            + s["processes"]["client"]["cpu_percent_one_logical_cpu"] for s in samples
        ])
        record["summary"]["bitbake_cpu_trimmed_mean_percent_one_logical_cpu"] = trimmed_mean([
            s["bitbake"]["cpu_percent_one_logical_cpu"] for s in samples
        ])
        record["summary"]["host_cpu_trimmed_mean_percent_total_capacity"] = trimmed_mean([
            s["host_cpu_percent_total_capacity"] for s in samples
        ])
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(record, indent=2) + "\n")
        print(json.dumps(record["summary"], indent=2))
    finally:
        if observer is not None:
            if job_id is not None and not cancellation["accepted"]:
                try:
                    observation.command(
                        9_999, None, {"type": "cancel_job", "job_id": job_id}
                    )
                except (OSError, RuntimeError, TimeoutError):
                    pass
            try: observer.close()
            except Exception: pass
        drain_stop.set()
        stop(client)
        stop(daemon)
        if master is not None:
            try: os.close(master)
            except OSError: pass
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
