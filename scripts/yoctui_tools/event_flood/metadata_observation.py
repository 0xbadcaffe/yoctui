def checked_instance(value: object) -> list[int]:
    if (
        not isinstance(value, list)
        or len(value) != 16
        or any(type(byte) is not int or not 0 <= byte <= 255 for byte in value)
    ):
        raise RuntimeError("daemon instance must contain the full 16-byte identity")
    return value


def wait_for_metadata_ready(
    client: ProtocolClient,
    snapshot: dict[str, object],
    build_dir: Path,
    timeout: float = 20.0,
    max_messages: int = 8192,
) -> int:
    """Read only, before measurement: require exact workspace and startup completion."""
    instance = checked_instance(client.daemon_instance_id)
    sequence = generation = -1
    workspace_seen = ready_seen = False

    def counter(value: object, name: str) -> int:
        if type(value) is not int or not 0 <= value < 2**64:
            raise RuntimeError(f"metadata readiness omitted a valid {name}")
        return value

    def workspace(event: dict[str, object]) -> None:
        nonlocal workspace_seen
        if event.get("type") != "workspace":
            return
        data = event.get("data")
        directory = data.get("build_dir") if isinstance(data, dict) else None
        if (
            not isinstance(directory, str)
            or Path(directory).resolve() != build_dir.resolve()
        ):
            raise RuntimeError("metadata workspace has the wrong build directory")
        workspace_seen = True

    def log(record: dict[str, object]) -> None:
        nonlocal ready_seen
        if record.get("source") != "daemon-metadata":
            return
        message = record.get("message", "")
        if not isinstance(message, str):
            raise RuntimeError("metadata readiness log is malformed")
        if message.startswith("Initial metadata ") or record.get("severity") == "error":
            raise RuntimeError(f"fixture startup metadata failed: {message[:1024]}")
        if message == "Initial workspace and recipe inventory ready":
            ready_seen = True

    def replacement(current: dict[str, object]) -> None:
        nonlocal sequence, generation, workspace_seen, ready_seen
        if checked_instance(current.get("daemon_instance_id")) != instance:
            raise RuntimeError("daemon instance changed during metadata readiness")
        next_sequence = counter(current.get("sequence"), "sequence")
        next_generation = counter(current.get("generation"), "generation")
        if next_sequence < sequence or next_generation < generation:
            raise RuntimeError("metadata snapshot sequence/generation regressed")
        sequence, generation = next_sequence, next_generation
        workspace_seen = ready_seen = False
        for record in current.get("recent_logs", []):
            log(record)
        for event in current.get("build_events", []):
            workspace(event)

    replacement(snapshot)
    deadline = time.monotonic() + timeout
    while not (workspace_seen and ready_seen):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise RuntimeError("fixture metadata readiness timed out")
        if max_messages <= 0:
            raise RuntimeError("fixture metadata readiness exceeded the message bound")
        try:
            message = client.receive(min(0.5, remaining))
        except EOFError as error:
            raise RuntimeError(
                "daemon disconnected during metadata readiness"
            ) from error
        if message is None:
            continue
        max_messages -= 1
        kind = message.get("type")
        if kind == "ping":
            client.send({"type": "pong", "nonce": message["nonce"]})
        elif kind == "snapshot":
            replacement(message)
        elif kind == "event":
            next_sequence = counter(message.get("sequence"), "sequence")
            next_generation = counter(message.get("generation"), "generation")
            if next_sequence != sequence + 1 or next_generation <= generation:
                raise RuntimeError(
                    "metadata event sequence/generation is not continuous"
                )
            sequence, generation = next_sequence, next_generation
            event = message.get("event", {})
            if event.get("type") == "build":
                workspace(event.get("data", {}))
            elif event.get("type") == "log":
                log(event.get("data", {}))
        else:
            raise RuntimeError(f"metadata readiness interrupted: {str(message)[:1024]}")
    return generation


def observe_build_ack(message: dict[str, object], request_id: int) -> bool:
    if message.get("type") in {"error", "detaching", "shutting_down"}:
        raise RuntimeError(f"fixture build protocol interrupted: {str(message)[:1024]}")
    if (
        message.get("type") != "command_result"
        or message.get("request_id") != request_id
    ):
        return False
    outcome = message.get("outcome", {})
    if not isinstance(outcome, dict) or outcome.get("type") != "accepted":
        raise RuntimeError(f"fixture build request rejected: {str(outcome)[:1024]}")
    return True
