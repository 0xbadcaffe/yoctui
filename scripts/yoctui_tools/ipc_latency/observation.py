class Observation:
    def __init__(
        self, client: ProtocolClient, observations: int, event_warmup: int
    ) -> None:
        self.client = client
        self.required = observations
        self.event_warmup = event_warmup
        self.protocol_sequences: list[int] = []
        self.event_samples: list[dict[str, int | float]] = []
        self.backend_disconnects = 0
        self.job_states: dict[int, dict[str, object]] = {}

    def process(self, message: dict[str, object], received_ns: int) -> None:
        if message.get("type") != "event":
            return
        sequence = message.get("sequence")
        if isinstance(sequence, int):
            self.protocol_sequences.append(sequence)
        event = message.get("event")
        job = job_event(message)
        if job is not None and isinstance(job.get("id"), int):
            self.job_states[int(job["id"])] = job
        if isinstance(event, dict) and event.get("type") == "build":
            data = event.get("data")
            if isinstance(data, dict) and data.get("type") == "disconnected":
                self.backend_disconnects += 1
        marker = latency_event(message)
        if marker is None or len(self.event_samples) >= self.required:
            return
        daemon_sequence, fixture_sequence, emitted_ns = marker
        if fixture_sequence <= self.event_warmup:
            return
        sequence = len(self.event_samples) + 1
        expected_fixture = self.event_warmup + sequence
        if fixture_sequence != expected_fixture:
            raise RuntimeError(
                "timestamped event order changed: "
                f"expected {expected_fixture}, got {fixture_sequence}"
            )
        if emitted_ns > received_ns:
            raise RuntimeError("event timestamp is later than client receipt")
        self.event_samples.append(
            {
                "sequence": sequence,
                "daemon_sequence": daemon_sequence,
                "fixture_sequence": fixture_sequence,
                "emitted_ns": emitted_ns,
                "received_ns": received_ns,
                "latency_ms": (received_ns - emitted_ns) / 1_000_000,
            }
        )

    def receive(self, timeout: float = 2.0) -> dict[str, object]:
        received = self.client.receive(timeout)
        if received is None:
            raise RuntimeError("timed out waiting for daemon IPC")
        message, received_ns = received
        self.process(message, received_ns)
        return message

    def command(
        self, request_id: int, generation: int | None, command: dict[str, object]
    ) -> tuple[dict[str, object], int, int]:
        sent_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
        self.client.send(
            {
                "type": "command",
                "request_id": request_id,
                "expected_generation": generation,
                "command": command,
            }
        )
        deadline = time.monotonic() + 2.0
        while time.monotonic() < deadline:
            received = self.client.receive(max(0.001, deadline - time.monotonic()))
            if received is None:
                break
            message, received_ns = received
            self.process(message, received_ns)
            if message.get("type") == "command_result" and message.get(
                "request_id"
            ) == request_id:
                return message, sent_ns, received_ns
        raise RuntimeError(f"timed out waiting for command result {request_id}")
