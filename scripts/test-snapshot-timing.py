#!/usr/bin/env python3
"""Check two production PTY attachments against an isolated timed IPC fixture.

This is client integration evidence, not live BitBake timing evidence.
"""
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import socket
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time

from terminal_capture import Screen


def send(connection, value):
    payload = json.dumps(value).encode()
    connection.sendall(struct.pack(">I", len(payload)) + payload)


def receive(connection):
    def exact(count):
        result = bytearray()
        while len(result) < count:
            chunk = connection.recv(count - len(result))
            if not chunk:
                raise EOFError
            result.extend(chunk)
        return result
    length = struct.unpack(">I", exact(4))[0]
    if length > 4 * 1024 * 1024:
        raise ValueError("oversized fixture request")
    return json.loads(exact(length))


def serve(listener, failures):
    try:
        for _ in range(2):
            connection, _ = listener.accept()
            with connection:
                connection.settimeout(15)
                hello = receive(connection)
                assert hello["type"] == "hello", hello
                send(connection, {
                    "type": "hello", "selected_version": {"major": 1, "minor": 2},
                    "daemon_instance_id": [80] * 16, "boot_id": "timing-fixture",
                    "capabilities": hello["capabilities"],
                    "limits": {
                        "maximum_frame_bytes": 4194304, "maximum_snapshot_bytes": 4194304,
                        "maximum_pending_requests": 64, "maximum_queue_depth": 256,
                        "maximum_terminal_rows": 512, "maximum_terminal_columns": 512,
                        "maximum_clients": 16, "maximum_pty_sessions": 16,
                        "maximum_scrollback_lines": 1000, "maximum_utility_output_bytes": 65536,
                    },
                })
                assert receive(connection)["type"] == "attach"
                send(connection, {
                    "type": "attached", "replayed_through": 5,
                    "snapshot": {
                        "daemon_instance_id": [80] * 16, "sequence": 5, "generation": 5,
                        "workspace": None, "project_profile": {"type": "not_loaded"},
                        "bitbake": {"lifecycle": "disconnected", "version": None, "capabilities": [], "diagnostic": None},
                        "jobs": [], "pty_sessions": [], "clients": [], "recent_logs": [], "recovery_warnings": [],
                        "build_events": [
                            {"type": "reset", "targets": ["timing-fixture-image"]},
                            {"type": "started", "started_unix_ms": 1000},
                            {"type": "task_completed", "recipe": "llvm-native", "task": "do_compile", "success": True, "started_unix_ms": 2000, "finished_unix_ms": 62000},
                            {"type": "completed", "success": True, "exit_code": 0, "finished_unix_ms": 65000},
                        ],
                    },
                })
                try:
                    while True:
                        message = receive(connection)
                        if message["type"] == "detach":
                            send(connection, {"type": "detaching"})
                            break
                except (EOFError, BrokenPipeError, ConnectionResetError):
                    # The capture terminates only its client after verifying
                    # the screen; it may close before reading our detach ack.
                    pass
    except Exception as error:
        failures.append(repr(error))


def capture(binary, root):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 160, 0, 0))
    env = {key: value for key, value in os.environ.items() if key not in ("BUILDDIR", "YOCTUI_BUILD_DIR", "PYTHONPATH")}
    env.update(TERM="xterm-256color", XDG_RUNTIME_DIR=str(root), XDG_CONFIG_HOME=str(root / "config"), XDG_STATE_HOME=str(root / "state"))
    def setup():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)
    process = subprocess.Popen([binary, "attach"], stdin=slave, stdout=slave, stderr=slave, env=env, preexec_fn=setup)
    os.close(slave)
    screen = Screen(160, 50)
    try:
        deadline = time.monotonic() + 8
        observed = None
        while time.monotonic() < deadline and process.poll() is None:
            if select.select([master], [], [], 0.1)[0]:
                try:
                    screen.feed(os.read(master, 65536))
                except OSError:
                    break
            if "00:01:04" in screen.text():
                if observed is None:
                    observed = time.monotonic()
                elif time.monotonic() - observed >= 1.2:
                    return screen.text()
        raise AssertionError("production attach did not retain fixed 64-second duration:\n" + screen.text())
    finally:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
        os.close(master)


def main():
    with tempfile.TemporaryDirectory(prefix="yoctui-timing-") as temporary:
        root = Path(temporary)
        directory = root / "yoctui"
        directory.mkdir(mode=0o700)
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
            listener.bind(str(directory / "daemon.sock"))
            (directory / "daemon.sock").chmod(0o600)
            listener.listen(2)
            listener.settimeout(20)
            failures = []
            server = threading.Thread(target=serve, args=(listener, failures), daemon=True)
            server.start()
            for _ in range(2):
                capture(sys.argv[1], root)
            server.join(timeout=5)
            assert not server.is_alive(), "fixture server did not finish"
            assert not failures, failures
    print("PASS: two production PTY attachments retain injected 64-second terminal duration")


if __name__ == "__main__":
    main()
