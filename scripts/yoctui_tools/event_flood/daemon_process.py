def start_daemon(
    binary: Path, root: Path, rate: int, duration: float
) -> tuple[subprocess.Popen[str], Path, Path]:
    runtime = root / "runtime"
    state = root / "state"
    config = root / "config"
    for directory in (runtime, state, config):
        directory.mkdir(mode=0o700)
    build, binary_dir = write_fake_environment(root)
    report = root / "generator.json"
    environment = os.environ.copy()
    environment.update(
        {
            "BUILDDIR": str(build),
            "YOCTUI_BUILD_DIR": str(build),
            "XDG_RUNTIME_DIR": str(runtime),
            "XDG_STATE_HOME": str(state),
            "XDG_CONFIG_HOME": str(config),
            "YOCTUI_BRIDGE_PATH": str(FIXTURE),
            "YOCTUI_PERF_EVENT_RATE": str(rate),
            "YOCTUI_PERF_EVENT_DURATION": str(duration),
            "YOCTUI_PERF_EVENT_PROFILE": "balanced",
            "YOCTUI_PERF_EVENT_TERMINAL": "success",
            "YOCTUI_PERF_EVENT_REPORT": str(report),
            "PATH": f"{binary_dir}:{environment['PATH']}",
        }
    )
    daemon = subprocess.Popen(
        [str(binary), "daemon", "foreground"],
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    socket_path = runtime / "yoctui/daemon.sock"
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        if socket_path.exists():
            return daemon, socket_path, report
        if daemon.poll() is not None:
            stdout, stderr = daemon.communicate()
            raise RuntimeError(f"daemon startup failed: {stdout}\n{stderr}")
        time.sleep(0.02)
    raise RuntimeError("daemon did not expose its socket within 20 seconds")


def stop_daemon(daemon: subprocess.Popen[str]) -> None:
    if daemon.poll() is None:
        daemon.send_signal(signal.SIGTERM)
        try:
            daemon.wait(timeout=5)
        except subprocess.TimeoutExpired:
            daemon.kill()
            daemon.wait(timeout=5)
    for stream in (daemon.stdout, daemon.stderr):
        if stream is not None:
            stream.close()
