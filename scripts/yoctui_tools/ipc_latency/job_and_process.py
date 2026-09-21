def accepted(result: dict[str, object]) -> bool:
    outcome = result.get("outcome")
    return isinstance(outcome, dict) and outcome.get("type") == "accepted"


def outcome_identity(result: dict[str, object]) -> tuple[str, str | None]:
    outcome = result.get("outcome")
    if not isinstance(outcome, dict) or not isinstance(outcome.get("type"), str):
        raise RuntimeError(f"command result omitted a typed outcome: {result}")
    code = outcome.get("code")
    return str(outcome["type"]), str(code) if isinstance(code, str) else None


def job_event(message: dict[str, object]) -> dict[str, object] | None:
    if message.get("type") != "event":
        return None
    event = message.get("event")
    if not isinstance(event, dict) or event.get("type") != "job_changed":
        return None
    data = event.get("data")
    return data if isinstance(data, dict) else None


def wait_for_job(
    observation: Observation, job_id: int | None, lifecycles: set[str]
) -> dict[str, object]:
    retained = next(
        (
            job
            for retained_id, job in observation.job_states.items()
            if (job_id is None or retained_id == job_id)
            and job.get("lifecycle") in lifecycles
        ),
        None,
    )
    if retained is not None:
        return retained
    deadline = time.monotonic() + 5.0
    while time.monotonic() < deadline:
        message = observation.receive(max(0.001, deadline - time.monotonic()))
        job = job_event(message)
        if job is not None and (job_id is None or job.get("id") == job_id):
            if job.get("lifecycle") in lifecycles:
                return job
    raise RuntimeError(f"timed out waiting for job {job_id} in {sorted(lifecycles)}")


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


def start_daemon(binary: Path, root: Path) -> tuple[subprocess.Popen[str], Path]:
    runtime = root / "runtime"
    state = root / "state"
    config = root / "config"
    for directory in (runtime, state, config):
        directory.mkdir(mode=0o700)
    build, binary_dir = write_fake_environment(root)
    environment = os.environ.copy()
    environment.update(
        {
            "BUILDDIR": str(build),
            "YOCTUI_BUILD_DIR": str(build),
            "XDG_RUNTIME_DIR": str(runtime),
            "XDG_STATE_HOME": str(state),
            "XDG_CONFIG_HOME": str(config),
            "YOCTUI_BRIDGE_PATH": str(BRIDGE),
            "YOCTUI_PERF_IPC_EVENT_RATE": "500",
            "PATH": f"{binary_dir}:{environment['PATH']}",
        }
    )
    daemon = subprocess.Popen(
        [str(binary), "daemon", "foreground"],
        cwd=ROOT,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    socket_path = runtime / "yoctui/daemon.sock"
    deadline = time.monotonic() + 20.0
    while time.monotonic() < deadline:
        if socket_path.exists():
            return daemon, socket_path
        if daemon.poll() is not None:
            stdout, stderr = daemon.communicate()
            raise RuntimeError(f"daemon startup failed: {stdout}\n{stderr}")
        time.sleep(0.02)
    raise RuntimeError("daemon did not expose its socket within 20 seconds")


def stop_process(process: subprocess.Popen[str], timeout: float = 5.0) -> None:
    if process.poll() is None:
        process.send_signal(signal.SIGTERM)
        try:
            process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=timeout)
    for stream in (process.stdout, process.stderr):
        if stream is not None:
            stream.close()


def wait_saturation_ready(process: subprocess.Popen[str], event_log: Path) -> None:
    deadline = time.monotonic() + 10.0
    while process.poll() is None and time.monotonic() < deadline:
        if event_log.exists() and '"event":"ready"' in event_log.read_text(
            encoding="utf-8"
        ):
            return
        time.sleep(0.02)
    raise RuntimeError("CPU saturation fixture did not become ready")
