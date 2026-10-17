#!/usr/bin/env python3
"""Check same-terminal desktop/console attachment and retained build timing.

This is client integration evidence, not live BitBake timing evidence.
"""
import fcntl
import json
import os
from pathlib import Path
import pty
import socket
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time

from pty_acceptance import TerminalAcceptance, isolated_environment


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
                # This fixture tests timing, not obsolete protocol negotiation.
                version = hello["maximum_version"]
                assert version["major"] == 1 and version["minor"] >= 3, hello
                send(connection, {
                    "type": "hello", "selected_version": version,
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


def capture(binary, root, environment, desktop):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 160, 0, 0))
    env = environment.copy()
    env["XDG_RUNTIME_DIR"] = str(root)
    # Do not mask an automatic desktop relaunch with a graphics override.
    env.pop("YOCTUI_TERMINAL_GRAPHICS", None)
    env.pop("DISPLAY", None)
    env.pop("WAYLAND_DISPLAY", None)
    env["TERM"] = "xterm-256color" if desktop else "linux"
    env["PATH"] = str(root / "bin") + os.pathsep + env.get("PATH", "")
    if desktop:
        env["DISPLAY"] = ":99"
    def setup():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)
    process = subprocess.Popen([binary, "attach"], stdin=slave, stdout=slave, stderr=slave, env=env, preexec_fn=setup)
    os.close(slave)
    terminal = TerminalAcceptance(master, process, bytearray(), 160, 50)
    try:
        deadline = time.monotonic() + 8
        observed = None
        while time.monotonic() < deadline and process.poll() is None:
            terminal.read(timeout=0.1)
            if "00:01:04" in terminal.screen.text():
                if observed is None:
                    observed = time.monotonic()
                elif time.monotonic() - observed >= 1.2:
                    return terminal.screen.text()
        raise AssertionError("production attach did not render timing in the original terminal:\n" + terminal.screen.text())
    finally:
        if process.poll() is None:
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
        environment = isolated_environment(root)
        bin_dir = root / "bin"
        bin_dir.mkdir()
        marker = root / "unexpected-xterm"
        xterm = bin_dir / "xterm"
        xterm.write_text('#!/bin/sh\n: > "$(dirname "$0")/../unexpected-xterm"\nexit 91\n')
        xterm.chmod(0o700)
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
            for desktop in (True, False):
                capture(sys.argv[1], root, environment, desktop)
                assert not marker.exists(), "attach unexpectedly launched another terminal"
            server.join(timeout=5)
            assert not server.is_alive(), "fixture server did not finish"
            assert not failures, failures
    print("PASS: desktop and text-console attach stay in the original PTY and retain build timing")


if __name__ == "__main__":
    main()
