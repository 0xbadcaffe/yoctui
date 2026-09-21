def process_rss(pid: int) -> int | None:
    try:
        fields = Path(f"/proc/{pid}/statm").read_text(encoding="utf-8").split()
        return int(fields[1]) * os.sysconf("SC_PAGE_SIZE")
    except (FileNotFoundError, IndexError, ValueError):
        return None


def process_cpu_seconds(pid: int) -> float | None:
    try:
        fields = Path(f"/proc/{pid}/stat").read_text(encoding="utf-8").split()
        ticks = int(fields[13]) + int(fields[14])
        return ticks / os.sysconf("SC_CLK_TCK")
    except (FileNotFoundError, IndexError, ValueError):
        return None


def classify_message(message: dict[str, object], observed: set[str]) -> None:
    if message.get("type") in {"snapshot", "attached"}:
        snapshot = message.get("snapshot")
        if isinstance(snapshot, dict):
            classify_snapshot(snapshot, observed)
        return
    if message.get("type") != "event":
        return
    event = message.get("event")
    if not isinstance(event, dict):
        return
    kind = event.get("type")
    data = event.get("data")
    if kind == "log" and isinstance(data, dict):
        classify_log(data, observed)
    elif kind == "build" and isinstance(data, dict):
        classify_build(data, observed)


def observe_pressure(message: dict[str, object], observed: dict[str, int]) -> None:
    if message.get("type") != "event":
        return
    event = message.get("event")
    if not isinstance(event, dict) or event.get("type") != "telemetry":
        return
    data = event.get("data")
    if not isinstance(data, dict):
        return
    pressure = data.get("pressure")
    if not isinstance(pressure, dict):
        return
    for key, value in pressure.items():
        if isinstance(value, int):
            observed[key] = max(observed.get(key, 0), value)


def classify_log(record: dict[str, object], observed: set[str]) -> None:
    message = record.get("message")
    if message == "PERF_CRITICAL_WARNING":
        observed.add("warning_sentinel")
    elif message == "PERF_CRITICAL_ERROR":
        observed.add("error_sentinel")
    elif message == "PERF_CRITICAL_CANCELLATION":
        observed.add("cancellation")


def classify_build(event: dict[str, object], observed: set[str]) -> None:
    kind = event.get("type")
    if event.get("recipe") == "perf-critical" and event.get("task") == "do_failure":
        mapping = {
            "task_queued": "critical_task_queued",
            "task_started": "critical_task_started",
            "task_progress": "critical_task_progress",
            "task_completed": "critical_task_failed",
        }
        if kind in mapping:
            observed.add(mapping[str(kind)])
    if kind == "completed":
        observed.add("build_terminal")
    elif kind == "disconnected":
        observed.add("backend_disconnect")


def classify_snapshot(snapshot: dict[str, object], observed: set[str]) -> None:
    for record in snapshot.get("recent_logs", []):
        if isinstance(record, dict):
            classify_log(record, observed)
    for event in snapshot.get("build_events", []):
        if isinstance(event, dict):
            classify_build(event, observed)


def write_fake_environment(root: Path) -> tuple[Path, Path]:
    build = root / "build"
    binary_dir = root / "bin"
    (build / "conf").mkdir(parents=True)
    binary_dir.mkdir()
    (build / "conf/local.conf").write_text(
        'MACHINE = "qemux86-64"\nDISTRO = "poky"\n', encoding="utf-8"
    )
    (build / "conf/bblayers.conf").write_text('BBLAYERS = ""\n', encoding="utf-8")
    bitbake = binary_dir / "bitbake"
    bitbake.write_text(
        "#!/bin/sh\n"
        'if [ "${1:-}" = --version ]; then\n'
        "  echo 'BitBake Build Tool Core version 2.18.0'\n"
        "else\n"
        "  echo 'usage: bitbake [options] target'\n"
        "fi\n",
        encoding="utf-8",
    )
    bitbake.chmod(0o755)
    return build, binary_dir
